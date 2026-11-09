use crate::app_commands::{AppState,MonthlySnapshot};
use payroll_core::Decimal;
use std::{collections::BTreeMap,fs::{self,File},io::Write,path::Path};
use tauri::State;
use zip::{write::SimpleFileOptions,CompressionMethod,ZipWriter};

#[derive(Clone)]
enum Cell { Text(String), Number(Decimal) }
impl From<&str> for Cell { fn from(value:&str)->Self{Self::Text(value.into())} }
impl From<String> for Cell { fn from(value:String)->Self{Self::Text(value)} }
impl From<Decimal> for Cell { fn from(value:Decimal)->Self{Self::Number(value)} }
type Rows=Vec<Vec<Cell>>;

fn escape(value:&str)->String{
    value.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&apos;")
}
fn column(mut number:usize)->String{
    let mut output=String::new();number+=1;
    while number>0{number-=1;output.insert(0,(b'A'+(number%26) as u8) as char);number/=26;}
    output
}
fn sheet_xml(rows:&Rows)->String{
    let mut xml=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>");
    for (row_index,row) in rows.iter().enumerate(){
        xml.push_str(&format!("<row r=\"{}\">",row_index+1));
        for (col_index,cell) in row.iter().enumerate(){
            let ref_id=format!("{}{}",column(col_index),row_index+1);
            match cell{
                Cell::Text(value)=>xml.push_str(&format!("<c r=\"{ref_id}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",escape(value))),
                Cell::Number(value)=>xml.push_str(&format!("<c r=\"{ref_id}\"><v>{value}</v></c>")),
            }
        }
        xml.push_str("</row>");
    }
    xml.push_str("</sheetData></worksheet>");xml
}
fn write_workbook(path:&Path,sheets:&[(String,Rows)])->Result<(),String>{
    if sheets.is_empty(){return Err("工作簿至少需要一张表".into());}
    let temporary=path.with_extension("xlsx.tmp");
    let result=(||{
        let file=File::create(&temporary).map_err(|e|e.to_string())?;
        let mut zip=ZipWriter::new(file);
        let options=SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut put=|name:&str,contents:String|->Result<(),String>{zip.start_file(name,options).map_err(|e|e.to_string())?;zip.write_all(contents.as_bytes()).map_err(|e|e.to_string())};
        let mut types=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>");
        for index in 1..=sheets.len(){types.push_str(&format!("<Override PartName=\"/xl/worksheets/sheet{index}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"));}
        types.push_str("</Types>");put("[Content_Types].xml",types)?;
        put("_rels/.rels","<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".into())?;
        let mut workbook=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>");
        let mut relationships=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
        for (index,(name,rows)) in sheets.iter().enumerate(){
            let number=index+1;
            workbook.push_str(&format!("<sheet name=\"{}\" sheetId=\"{number}\" r:id=\"rId{number}\"/>",escape(name)));
            relationships.push_str(&format!("<Relationship Id=\"rId{number}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{number}.xml\"/>"));
            put(&format!("xl/worksheets/sheet{number}.xml"),sheet_xml(rows))?;
        }
        workbook.push_str("</sheets></workbook>");relationships.push_str("</Relationships>");
        put("xl/workbook.xml",workbook)?;put("xl/_rels/workbook.xml.rels",relationships)?;
        zip.finish().map_err(|e|e.to_string())?;
        fs::rename(&temporary,path).map_err(|e|e.to_string())
    })();
    if result.is_err(){let _=fs::remove_file(temporary);}
    result
}
fn headers(values:&[&str])->Vec<Cell>{values.iter().map(|value|Cell::from(*value)).collect()}
fn sheets(names:&BTreeMap<String,String>,versions:&[MonthlySnapshot])->Vec<(String,Rows)>{
    let mut detail=vec![headers(&["工号","姓名","固定薪","绩效","调整项","应发工资","员工承担","本期预扣个税","实发工资","单位成本"])];
    let mut summary:BTreeMap<String,Decimal>=BTreeMap::new();
    let mut issues=vec![headers(&["工号","姓名","级别","说明"])];
    let mut explanation=vec![headers(&["工号","姓名","工资项目","公式或口径","输入来源","结果"])];
    let mut payslips=vec![headers(&["工号","姓名","固定薪","绩效","调整项","应发工资","员工承担","个税","实发工资"])];
    for v in versions{
        let name=names.get(&v.employee_id).map(String::as_str).unwrap_or("");
        let tax=v.result.withholding.as_ref().map(|w|w.current_withholding.to_string()).unwrap_or_else(||"不可准确测算".into());
        let net=v.result.net.map(|n|n.to_string()).unwrap_or_else(||"不可准确测算".into());
        detail.push(vec![v.employee_id.as_str().into(),name.into(),v.fixed_salary.into(),v.performance.into(),v.adjustment.into(),v.result.gross.into(),v.result.employee_deductions.into(),tax.as_str().into(),net.as_str().into(),v.result.employer_cost.into()]);
        payslips.push(vec![v.employee_id.as_str().into(),name.into(),v.fixed_salary.into(),v.performance.into(),v.adjustment.into(),v.result.gross.into(),v.result.employee_deductions.into(),tax.as_str().into(),net.as_str().into()]);
        for (item,value) in [("固定薪",v.fixed_salary),("绩效",v.performance),("调整项",v.adjustment),("应发工资",v.result.gross),("员工承担",v.result.employee_deductions),("单位成本",v.result.employer_cost)]{
            *summary.entry(item.into()).or_default()+=value;
        }
        if v.result.net.is_none(){issues.push(vec![v.employee_id.as_str().into(),name.into(),"阻断".into(),"税务基线或本期扣除缺失，个税与实发不可准确测算".into()]);}
        if v.result.net.is_some_and(|value|value<Decimal::ZERO){issues.push(vec![v.employee_id.as_str().into(),name.into(),"警告".into(),"实发工资为负，请复核补扣款".into()]);}
        explanation.push(vec![v.employee_id.as_str().into(),name.into(),"固定薪".into(),"固定月薪 ÷ 标准工作日 × 实出勤日；两位小数".into(),format!("标准 {} 日，实际 {} 日",v.input.standard_days,v.input.actual_days).into(),v.fixed_salary.into()]);
        match &v.contributions {
            Some(rows)=>for row in rows { for (label,percent,amount) in [("员工",&row.employee_percent,row.employee_amount),("单位",&row.employer_percent,row.employer_amount)] {explanation.push(vec![v.employee_id.as_str().into(),name.into(),format!("{} · {label}承担",row.project).into(),format!("{} × {}%（逐项目到分）",row.base.value,percent.value).into(),format!("基数：{}；比例：{}",if row.base.scope=="employee"{"员工覆盖"}else{"主体默认"},if percent.scope=="employee"{"员工覆盖"}else{"主体默认"}).into(),amount.into()]);}},
            None=>explanation.push(vec![v.employee_id.as_str().into(),name.into(),"社保公积金".into(),"旧版金额结果，无基数比例快照".into(),"历史快照".into(),v.result.employee_deductions.into()]),
        }
        for trace in &v.rule_trace{explanation.push(vec![v.employee_id.as_str().into(),name.into(),trace.code.as_str().into(),trace.expression.as_str().into(),format!("{:?}",trace.inputs).into(),trace.result.into()]);}
        explanation.push(vec![v.employee_id.as_str().into(),name.into(),"累计预扣个税".into(),"累计应纳税所得额 × 预扣率 − 速算扣除数 − 此前实际已预扣税".into(),v.input.tax_baseline.as_ref().map(|b|format!("历史实际已预扣 {}",b.cumulative_withheld_tax)).unwrap_or_else(||"缺历史基线".into()).into(),tax.into()]);
    }
    let mut totals=vec![headers(&["工资项目","合计金额"])];
    for (item,value) in summary{totals.push(vec![item.into(),value.into()]);}
    let output=vec![("工资明细".into(),detail),("项目汇总".into(),totals),("异常清单".into(),issues),("计算说明".into(),explanation),("员工工资单".into(),payslips)];
    output
}

#[tauri::command]
pub fn exports_payroll_xlsx(state:State<AppState>,path:String,password:String,run_id:String)->Result<usize,String>{
    state.with_session_mut(|session|{
        let verification=state.store.unlock(&session.subject.id,&password)?;drop(verification);
        let (run,versions)=crate::export_runs::selected_snapshots(session,&run_id)?;
        let names=crate::export_runs::employee_names(session,&versions)?;
        write_workbook(Path::new(&path),&sheets(&names,&versions))?;
        session.log_action("export_payroll_xlsx",&serde_json::json!({"run_id":run.id,"run_version":run.version,"version_ids":run.version_ids}).to_string())?;
        Ok(versions.len())
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn workbook_opens_with_all_five_sheets(){
        let path=std::env::temp_dir().join(format!("salary-export-{}.xlsx",uuid::Uuid::new_v4()));
        write_workbook(&path,&[("工资明细".into(),vec![headers(&["主体","金额"]),vec!["测试&甲".into(),Decimal::new(12345,2).into()]]),("项目汇总".into(),vec![headers(&["项目"])]),("异常清单".into(),vec![headers(&["说明"])]),("计算说明".into(),vec![headers(&["公式"])]),("员工工资单".into(),vec![headers(&["员工"])])]).unwrap();
        let parsed=payroll_import::read_file(&path).unwrap();
        assert_eq!(parsed.sheets.len(),5);
        assert_eq!(parsed.sheets[0].rows[0][0],"测试&甲");
        assert_eq!(parsed.sheets[0].rows[0][1],"123.45");
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn report_has_business_columns_without_metadata(){
        let report=sheets(&BTreeMap::new(),&[]);
        assert_eq!(report.len(),5);
        for (name,rows) in report{
            assert_eq!(rows.len(),1);
            let xml=sheet_xml(&rows);
            for field in ["主体","所属期","计划发放月","试算版本","计算结果ID","生成时间","用途标识"]{assert!(!xml.contains(field),"{name} contains {field}");}
            if name!="项目汇总"{assert!(xml.contains("工号"));assert!(xml.contains("姓名"));}
        }
    }
}
