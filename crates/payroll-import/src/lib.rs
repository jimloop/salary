//! Spreadsheet reading and deterministic row validation. Does not write to storage.
use calamine::{open_workbook_auto, Reader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all="snake_case")]
pub enum ImportKind { Employees, Attendance, Performance, Adjustments, TaxHistory, Social, HistoricalPayroll, SalaryPlans, Bonus }
impl ImportKind {
    pub fn required(self) -> &'static [&'static str] {
        match self {
            Self::SalaryPlans => &["employee_id","effective_from","base_salary"],
            Self::Bonus => &["employee_id","bonus_year","target","performance_factor","eligible_days","plan_days"],
            Self::Employees => &["employee_id","name","base_salary"],
            Self::Attendance => &["employee_id","period","standard_days","actual_days"],
            Self::Performance => &["employee_id","period","performance"],
            Self::Adjustments => &["employee_id","period","adjustment"],
            Self::TaxHistory => &["employee_id","payout_month","cumulative_income","cumulative_withheld_tax"],
            Self::Social => &["employee_id","project","start"],
            Self::HistoricalPayroll => &["employee_id","period","gross","net"],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormulaCell { pub row:u32, pub column:u32, pub expression:String }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sheet { pub name:String, pub headers:Vec<String>, pub rows:Vec<Vec<String>>, pub formulas:Vec<FormulaCell> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilePreview { pub filename:String, pub sha256:String, pub sheets:Vec<Sheet> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RowError { pub sheet:String, pub row:usize, pub field:String, pub reason:String }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedRow { pub sheet:String, pub row:usize, pub fields:BTreeMap<String,String> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Validation { pub rows:Vec<ValidatedRow>, pub errors:Vec<RowError> }

fn valid_month(value:&str)->bool{
    let bytes=value.as_bytes();
    bytes.len()==7 && bytes[4]==b'-' && bytes.iter().enumerate().all(|(i,b)|i==4||b.is_ascii_digit())
        && value[..4].parse::<u32>().is_ok_and(|year|year>0)
        && value[5..7].parse::<u8>().is_ok_and(|month|(1..=12).contains(&month))
}
fn valid_date(value:&str)->bool{
    let bytes=value.as_bytes();
    if bytes.len()!=10||bytes[7]!=b'-'||!bytes[..7].iter().all(u8::is_ascii)||!valid_month(&value[..7])||!bytes[8..10].iter().all(u8::is_ascii_digit){return false;}
    let year=value[..4].parse::<u32>().unwrap_or(0);
    let month=value[5..7].parse::<u8>().unwrap_or(0);
    let day=value[8..10].parse::<u8>().unwrap_or(0);
    let leap=year%4==0&&(year%100!=0||year%400==0);
    let days=match month{2=>if leap{29}else{28},4|6|9|11=>30,_=>31};
    year>0&&day>=1&&day<=days
}

pub fn read_file(path:&Path)->Result<FilePreview,String> {
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    if bytes.len()>100_000_000 {return Err("文件超过 100 MB 上限".into());}
    let sha256=format!("{:x}",Sha256::digest(&bytes));
    let filename=path.file_name().and_then(|v|v.to_str()).ok_or("文件名无效")?.to_owned();
    let extension=path.extension().and_then(|e|e.to_str()).unwrap_or("").to_ascii_lowercase();
    let sheets=match extension.as_str(){
        "csv"=>{
            let mut reader=csv::ReaderBuilder::new().flexible(true).from_reader(bytes.as_slice());
            let headers=reader.headers().map_err(|e|e.to_string())?.iter().map(|v|v.trim_start_matches('\u{feff}').to_string()).collect();
            let mut rows=Vec::new();
            for record in reader.records(){let record=record.map_err(|e|e.to_string())?;rows.push(record.iter().map(str::to_string).collect());if rows.len()>100_000{return Err("文件超过 10 万数据行上限".into());}}
            vec![Sheet{name:"CSV".into(),headers,rows,formulas:Vec::new()}]
        },
        "xlsx"|"xls"|"xlsb"|"ods"=>{
            let mut book=open_workbook_auto(path).map_err(|e|e.to_string())?;
            let mut result=Vec::new();
            for name in book.sheet_names().to_vec(){
                let range=book.worksheet_range(&name).map_err(|e|e.to_string())?;
                let formula_range=book.worksheet_formula(&name).map_err(|e|e.to_string())?;
                let (start_row,start_col)=formula_range.start().unwrap_or((0,0));
                let formulas=formula_range.used_cells().map(|(row,column,expression)|FormulaCell{row:start_row+row as u32+1,column:start_col+column as u32+1,expression:expression.clone()}).collect();
                let mut iter=range.rows();
                let headers=iter.next().map(|row|row.iter().map(|cell|cell.to_string()).collect()).unwrap_or_default();
                let mut rows=Vec::new();
                for row in iter{rows.push(row.iter().map(|cell|cell.to_string()).collect());if rows.len()>100_000{return Err(format!("工作表 {name} 超过 10 万数据行上限"));}}
                result.push(Sheet{name,headers,rows,formulas});
            }
            result
        },
        _=>return Err("仅支持 CSV、XLSX、XLS、XLSB 或 ODS 文件".into()),
    };
    if sheets.is_empty(){return Err("文件中没有工作表".into());}
    Ok(FilePreview{filename,sha256,sheets})
}

pub fn validate(preview:&FilePreview,kind:ImportKind,subject_id:&str,sheet_name:&str,mapping:&BTreeMap<String,String>)->Result<Validation,String>{
    if kind==ImportKind::Social && (mapping.contains_key("employee_contribution") || mapping.contains_key("employer_contribution")) { return Err("旧金额表不能用于新计算，请下载员工社保公积金配置模板".into()); }
    let sheet=preview.sheets.iter().find(|s|s.name==sheet_name).ok_or("工作表不存在")?;
    for field in kind.required(){if mapping.get(*field).is_none_or(|value|value.is_empty()){return Err(format!("缺少必填字段映射：{field}"));}}
    for source in mapping.values().filter(|v|!v.is_empty()){if !sheet.headers.contains(source){return Err(format!("原始列不存在：{source}"));}}
    let mut errors=Vec::new();let mut rows=Vec::new();let mut seen=BTreeSet::new();
    for (index,raw) in sheet.rows.iter().enumerate(){
        let row_no=index+2;
        if raw.iter().all(|v|v.trim().is_empty()){continue;}
        let mut fields:BTreeMap<String,String>=mapping.iter().filter(|(_,original)|!original.is_empty()).map(|(standard,original)|{
            let col=sheet.headers.iter().position(|h|h==original).unwrap();
            (standard.clone(),raw.get(col).map(|s|s.trim()).unwrap_or("").to_owned())
        }).collect();
        fields.insert("subject_id".into(),subject_id.into());
        for field in kind.required(){if fields.get(*field).is_none_or(|v|v.is_empty()){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:(*field).into(),reason:"必填字段为空，请在源文件补全".into()});}}
        let numeric:&[&str]=match kind { ImportKind::SalaryPlans=>&["base_salary"],ImportKind::Bonus=>&["target","performance_factor","eligible_days","plan_days","minimum","maximum"],ImportKind::Employees=>&["base_salary"],ImportKind::Attendance=>&["standard_days","actual_days","additional_deduction"],ImportKind::Performance=>&["performance"],ImportKind::Adjustments=>&["adjustment"],ImportKind::TaxHistory=>&["cumulative_income","cumulative_exempt_income","cumulative_basic_deduction","cumulative_special_deduction","cumulative_additional_deduction","cumulative_other_deduction","cumulative_tax_relief","cumulative_withheld_tax"],ImportKind::Social=>&["base","employee_percent","employer_percent"],ImportKind::HistoricalPayroll=>&["gross","net"] };
        for field in numeric{if let Some(value)=fields.get(*field){if !value.is_empty()&&Decimal::from_str(&value.replace(',',"")).is_err(){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:(*field).into(),reason:"金额或数量格式无效，请填写数字".into()});}}}
        for field in ["period","payout_month","start","end"]{
            if let Some(value)=fields.get(field){if !value.is_empty()&&!valid_month(value){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:field.into(),reason:"月份格式无效，请使用 YYYY-MM".into()});}}
        }
        for field in ["hire_date","effective_from","effective_to"]{
            if let Some(value)=fields.get(field){if !value.is_empty()&&!(valid_date(value)||kind==ImportKind::SalaryPlans&&valid_month(value)){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:field.into(),reason:"日期无效，请使用 YYYY-MM-DD".into()});}}
        }
        for field in ["base_salary","standard_days","actual_days","additional_deduction","employee_contribution","employer_contribution","cumulative_income","cumulative_withheld_tax"]{
            if let Some(value)=fields.get(field){if let Ok(number)=Decimal::from_str(&value.replace(',',"")){if number.is_sign_negative()||field=="standard_days"&&number.is_zero(){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:field.into(),reason:"该值必须为非负数，标准工作日须大于零".into()});}}}
        }
        if let (Some(standard),Some(actual))=(fields.get("standard_days"),fields.get("actual_days")){
            if let (Ok(standard),Ok(actual))=(Decimal::from_str(standard),Decimal::from_str(actual)){
                if actual>standard{errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"actual_days".into(),reason:"实出勤日不能超过标准工作日".into()});}
            }
        }
        if kind==ImportKind::SalaryPlans&&fields.get("effective_to").is_some_and(|end|!end.is_empty()&&fields.get("effective_from").is_some_and(|start|end<start)){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"effective_to".into(),reason:"结束日期早于生效日期".into()});}
        if kind==ImportKind::Bonus {
            let days=fields.get("eligible_days").and_then(|v|v.parse::<u32>().ok());let total=fields.get("plan_days").and_then(|v|v.parse::<u32>().ok());
            if days.zip(total).is_none_or(|(days,total)|total==0||days>total){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"eligible_days".into(),reason:"资格天数及周期天数必须为整数，且 0 ≤ 资格天数 ≤ 周期天数、周期大于零".into()});}
            if fields.get("bonus_year").is_none_or(|v|v.len()!=4||v.parse::<u32>().is_err()){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"bonus_year".into(),reason:"奖金年度须为四位年份".into()});}
            if let (Some(min),Some(max))=(fields.get("minimum").and_then(|v|v.parse::<Decimal>().ok()),fields.get("maximum").and_then(|v|v.parse::<Decimal>().ok())){if min>max{errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"maximum".into(),reason:"封顶金额不能低于保底金额".into()});}}
            for field in ["target","performance_factor","minimum","maximum"]{if fields.get(field).and_then(|v|v.parse::<Decimal>().ok()).is_some_and(|v|v.is_sign_negative()){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:field.into(),reason:"金额和系数不能为负".into()});}}
        }
        if kind==ImportKind::Social {
            if ["base","employee_percent","employer_percent"].iter().all(|key|fields.get(*key).is_none_or(|v|v.is_empty())) { errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"base".into(),reason:"至少填写基数或一方比例".into()}); }
            for key in ["base","employee_percent","employer_percent"] { if fields.get(key).and_then(|v|v.parse::<Decimal>().ok()).is_some_and(|n|if key=="base" {n<=Decimal::ZERO}else{n<Decimal::ZERO||n>Decimal::from(100)}) { errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:key.into(),reason:"基数须为正数，比例须在 0%–100% 之间".into()}); } }
            if fields.get("end").is_some_and(|end|!end.is_empty()&&fields.get("start").is_some_and(|start|end<start)){errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"end".into(),reason:"截止月份早于生效月份".into()});}
        }
        let key=format!("{}:{}",fields.get("employee_id").unwrap_or(&String::new()),fields.get("period").or_else(||fields.get("payout_month")).or_else(||fields.get("effective_from")).or_else(||fields.get("bonus_year")).unwrap_or(&String::new()));
        let key=if kind==ImportKind::Social{format!("{key}:{}:{}",fields.get("project").cloned().unwrap_or_default(),fields.get("start").cloned().unwrap_or_default())}else{key};
        if !seen.insert(key) {errors.push(RowError{sheet:sheet_name.into(),row:row_no,field:"employee_id".into(),reason:"文件中工号与期间重复".into()});}
        for field in numeric {if let Some(value)=fields.get_mut(*field){*value=value.replace(',',"");}}
        rows.push(ValidatedRow{sheet:sheet_name.into(),row:row_no,fields});
    }
    Ok(Validation{rows,errors})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn accepts_rows_without_subject_column() {
        let preview=FilePreview{filename:"x.csv".into(),sha256:"a".into(),sheets:vec![Sheet{name:"CSV".into(),headers:vec!["工号".into(),"姓名".into(),"固定月薪".into()],rows:vec![vec!["01".into(),"甲".into(),"100".into()]],formulas:vec![]}]};
        let mapping=BTreeMap::from([("employee_id".into(),"工号".into()),("name".into(),"姓名".into()),("base_salary".into(),"固定月薪".into())]);
        let result=validate(&preview,ImportKind::Employees,"current-subject","CSV",&mapping).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.rows[0].fields["subject_id"],"current-subject");
    }
    #[test] fn binds_current_subject_and_detects_duplicate() {
        let preview=FilePreview{filename:"x.csv".into(),sha256:"a".into(),sheets:vec![Sheet{name:"CSV".into(),headers:vec!["主体ID".into(),"工号".into(),"姓名".into(),"固定月薪".into()],rows:vec![vec!["B".into(),"01".into(),"甲".into(),"100".into()],vec!["A".into(),"01".into(),"乙".into(),"200".into()]],formulas:vec![]}]};
        let mapping=BTreeMap::from([("subject_id".into(),"主体ID".into()),("employee_id".into(),"工号".into()),("name".into(),"姓名".into()),("base_salary".into(),"固定月薪".into())]);
        let result=validate(&preview,ImportKind::Employees,"A","CSV",&mapping).unwrap();
        assert_eq!(result.errors.len(),1);
        assert!(result.rows.iter().all(|row|row.fields["subject_id"]=="A"));
    }
    #[test] fn reads_mock_csv_and_maps_all_rows() {
        let path=Path::new("../../tests/fixtures/mock-payroll/E-A/attendance.csv");
        let file=read_file(path).unwrap();
        assert_eq!(file.sheets[0].rows.len(),4);
        let mapping=BTreeMap::from([("subject_id".into(),"主体ID".into()),("employee_id".into(),"工号".into()),("period".into(),"所属期".into()),("standard_days".into(),"标准工作日".into()),("actual_days".into(),"实出勤日".into())]);
        let valid=validate(&file,ImportKind::Attendance,"E-A","CSV",&mapping).unwrap();
        assert!(valid.errors.is_empty(),"{:?}",valid.errors);
        assert_eq!(valid.rows.len(),4);
        let wrong=validate(&file,ImportKind::Attendance,"E-B","CSV",&mapping).unwrap();
        assert!(wrong.errors.is_empty());
        assert!(wrong.rows.iter().all(|row|row.fields["subject_id"]=="E-B"));
    }
    #[test] fn rejects_invalid_dates_and_attendance_ranges(){
        assert!(!valid_month("2026-13"));
        assert!(!valid_date("2026-02-29"));
        assert!(valid_date("2024-02-29"));
        let preview=FilePreview{filename:"attendance.csv".into(),sha256:"x".into(),sheets:vec![Sheet{name:"CSV".into(),headers:vec!["主体".into(),"工号".into(),"月份".into(),"标准".into(),"实出勤".into()],rows:vec![vec!["A".into(),"E1".into(),"2026-13".into(),"22".into(),"23".into()]],formulas:vec![]}]};
        let mapping=BTreeMap::from([("subject_id".into(),"主体".into()),("employee_id".into(),"工号".into()),("period".into(),"月份".into()),("standard_days".into(),"标准".into()),("actual_days".into(),"实出勤".into())]);
        let result=validate(&preview,ImportKind::Attendance,"A","CSV",&mapping).unwrap();
        assert!(result.errors.iter().any(|e|e.field=="period"));
        assert!(result.errors.iter().any(|e|e.field=="actual_days"));
    }
    #[test] fn parses_one_hundred_thousand_rows_without_truncation(){
        let path=std::env::temp_dir().join(format!("salary-import-{}.csv",std::process::id()));
        let mut bytes=b"\xEF\xBB\xBFsubject_id,employee_id,period,standard_days,actual_days\n".to_vec();
        for index in 0..100_000{
            bytes.extend_from_slice(format!("A,E{:04},2026-10,22,21\n",index%1000).as_bytes());
        }
        fs::write(&path,bytes).unwrap();
        let file=read_file(&path).unwrap();
        assert_eq!(file.sheets[0].rows.len(),100_000);
        fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod configuration_tests {
 use super::*;
 #[test]
 fn contribution_rows_allow_partial_and_zero_but_reject_old_amounts_and_invalid_values(){
  let headers:Vec<String>=["employee_id","project","start","end","base","employee_percent","employer_percent"].iter().map(|v|v.to_string()).collect();let mapping=headers.iter().map(|key|(key.clone(),key.clone())).collect();
  let mut preview=FilePreview{filename:"config.csv".into(),sha256:"fake".into(),sheets:vec![Sheet{name:"CSV".into(),headers,rows:vec![vec!["E1".into(),"住房公积金".into(),"2026-01".into(),"".into(),"".into(),"0".into(),"".into()],vec!["E1".into(),"养老保险".into(),"2026-01".into(),"".into(),"10000".into(),"".into(),"".into()]],formulas:vec![]}]};
  let valid=validate(&preview,ImportKind::Social,"subject","CSV",&mapping).unwrap();assert!(valid.errors.is_empty());assert_eq!(valid.rows[0].fields["employee_percent"],"0");assert_eq!(valid.rows[0].fields["base"],"");assert_eq!(valid.rows[0].fields["subject_id"],"subject");
  preview.sheets[0].rows[0][5]="101".into();preview.sheets[0].rows[1][4]="0".into();assert_eq!(validate(&preview,ImportKind::Social,"subject","CSV",&mapping).unwrap().errors.len(),2);
  let old_mapping=BTreeMap::from([("employee_contribution".into(),"员工承担".into())]);assert!(validate(&preview,ImportKind::Social,"subject","CSV",&old_mapping).unwrap_err().contains("旧金额表"));
 }
}
