use crate::app_commands::{amount,AppState,Employee,MonthlyRequest,MonthlySnapshot,SalaryPlan};
use payroll_core::{monthly::{calculate_pay,CurrentTaxInput,TaxBaseline},Decimal};
use payroll_import::ValidatedRow;
use payroll_storage::Collection;
use serde::{Deserialize,Serialize};
use std::collections::BTreeMap;
use tauri::State;
use uuid::Uuid;
use crate::export_runs::CalculationRun;

#[derive(Clone,Deserialize)]
pub struct BatchRequest { pub period:String,pub payout_month:String,pub region:String,pub basic_deduction:String,pub other_deduction:String,pub exempt_income:String,pub tax_relief:String }
#[derive(Serialize)]
pub struct BatchResult { pub run_id:String,pub run_version:u32,pub version_ids:Vec<String>,pub warnings:Vec<String> }

fn rows(session:&payroll_storage::Session,kind:&str)->Result<Vec<ValidatedRow>,String>{
    session.imported_rows(kind)?.into_iter().map(|v|serde_json::from_value(v).map_err(|e|e.to_string())).collect()
}
fn index(rows:Vec<ValidatedRow>,date_field:&str,date:&str)->Result<BTreeMap<String,ValidatedRow>,String>{
    let mut result=BTreeMap::new();
    for row in rows {if row.fields.get(date_field).is_some_and(|v|v==date){if let Some(id)=row.fields.get("employee_id").cloned(){
        if result.insert(id.clone(),row).is_some(){return Err(format!("工号 {id} 在 {date} 有重复导入记录；请先核对批次，避免静默覆盖"));}
    }}}
    Ok(result)
}
fn number(row:&ValidatedRow,key:&str)->Result<Decimal,String>{amount(row.fields.get(key).ok_or_else(||format!("{} 第 {} 行缺少 {key}",row.sheet,row.row))?,key)}
pub(crate) fn tax_baseline(row:&ValidatedRow)->Option<TaxBaseline>{
    let n=|key|row.fields.get(key).and_then(|v|v.parse::<Decimal>().ok());
    Some(TaxBaseline{cumulative_income:n("cumulative_income")?,cumulative_exempt_income:n("cumulative_exempt_income")?,cumulative_basic_deduction:n("cumulative_basic_deduction")?,cumulative_special_deduction:n("cumulative_special_deduction")?,cumulative_additional_deduction:n("cumulative_additional_deduction")?,cumulative_other_deduction:n("cumulative_other_deduction")?,cumulative_tax_relief:n("cumulative_tax_relief")?,cumulative_withheld_tax:n("cumulative_withheld_tax")?})
}
pub(crate) fn salary_for(session:&payroll_storage::Session,employee:&Employee,period:&str)->Result<Decimal,String>{
    let start=format!("{period}-01");let mut selected:Option<SalaryPlan>=None;let mut prior_plan=false;
    {
        for plan in crate::import_inputs::plans(session)?.into_iter().filter(|p|p.employee_id==employee.id){
            if plan.effective_from<=start{prior_plan=true;}
            if plan.effective_from.starts_with(period)&&plan.effective_from!=start{return Err(format!("工号 {} 在 {} 月中调薪，需分段考勤输入",employee.id,period));}
            if plan.effective_to.as_deref().is_some_and(|end|end.starts_with(period)){return Err(format!("工号 {} 的薪酬方案在 {} 月中结束，需分段考勤输入",employee.id,period));}
            if plan.effective_from<=start&&plan.effective_to.as_deref().is_none_or(|end|end>=start.as_str()){if selected.is_some(){return Err(format!("工号 {} 有重叠的生效薪酬方案，请核对导入批次",employee.id));}selected=Some(plan);}
        }
    }
    if prior_plan&&selected.is_none(){return Err(format!("工号 {} 在 {} 没有生效薪酬方案",employee.id,period));}
    amount(selected.as_ref().map(|p|p.base_salary.as_str()).unwrap_or(&employee.base_salary),"固定月薪")
}
#[tauri::command]
pub fn payroll_calculate_batch(state:State<AppState>,request:BatchRequest)->Result<BatchResult,String>{
    state.with_session_mut(|session|run(session,request))
}
fn run(session:&mut payroll_storage::Session,request:BatchRequest)->Result<BatchResult,String>{
        let attendance=index(rows(session,"Attendance")?,"period",&request.period)?;
        if attendance.is_empty(){return Err("当前所属期没有已导入的考勤数据".into());}
        let performance=index(rows(session,"Performance")?,"period",&request.period)?;
        let adjustments=index(rows(session,"Adjustments")?,"period",&request.period)?;
        let social=index(rows(session,"Social")?,"period",&request.period)?;
        let tax=index(rows(session,"TaxHistory")?,"payout_month",&request.payout_month)?;
        let region=if request.region.is_empty(){session.subject.region.clone()}else{request.region.clone()};
        let selected_rules=crate::payroll_rules::selected(&crate::payroll_rules::all(session)?,&format!("{}-01",request.period),&region);
        let mut records=Vec::new();let mut ids=Vec::new();let mut warnings=Vec::new();
        for (employee_id,att) in attendance{
            let employee:Employee=session.get(Collection::Employees,&employee_id)?.ok_or_else(||format!("考勤第 {} 行工号 {} 不在当前主体员工档案",att.row,employee_id))?;
            let perf=performance.get(&employee_id).ok_or_else(||format!("工号 {employee_id} 缺少绩效输入；请显式导入 0"))?;
            let adj=adjustments.get(&employee_id).ok_or_else(||format!("工号 {employee_id} 缺少补扣款输入；请显式导入 0"))?;
            let social_row=social.get(&employee_id).ok_or_else(||format!("工号 {employee_id} 缺少社保公积金输入"))?;
            let base=salary_for(session,&employee,&request.period)?;
            let standard=number(&att,"standard_days")?;let actual=number(&att,"actual_days")?;
            if standard<=Decimal::ZERO||actual<Decimal::ZERO||actual>standard{return Err(format!("工号 {employee_id} 考勤天数无效"));}
            let performance_amount=number(perf,"performance")?;let adjustment=number(adj,"adjustment")?;
            let deductions=number(social_row,"employee_contribution")?;let employer=number(social_row,"employer_contribution")?;
            let fields=BTreeMap::from([("base_salary".into(),base),("standard_days".into(),standard),("actual_days".into(),actual),("performance".into(),performance_amount),("adjustment".into(),adjustment),("employee_contribution".into(),deductions),("employer_contribution".into(),employer)]);
            let performance_input=performance_amount;let adjustment_input=adjustment;let contribution_input=deductions;let employer_input=employer;
            let values=crate::payroll_rules::calculate(&selected_rules,&fields,&format!("{}-01",request.period),&region)?;
            let fixed=values.fixed;let performance_amount=values.performance;let adjustment=values.adjustment;let deductions=values.deductions;let employer=values.employer;let gross=values.gross;let trace=values.trace;
            let baseline=tax.get(&employee_id).and_then(tax_baseline);
            let additional=att.fields.get("additional_deduction").and_then(|v|v.parse::<Decimal>().ok());
            if baseline.is_none()||additional.is_none(){warnings.push(format!("工号 {employee_id} 缺少完整税务历史或本期专项附加扣除；个税和实发不可准确测算"));}
            let current=baseline.as_ref().zip(additional).map(|(_,additional)|Ok::<CurrentTaxInput,String>(CurrentTaxInput{taxable_income:gross,exempt_income:amount(&request.exempt_income,"本期免税收入")?,basic_deduction:amount(&request.basic_deduction,"基本减除费用")?,special_deduction:deductions,additional_deduction:additional,other_deduction:amount(&request.other_deduction,"其他扣除")?,tax_relief:amount(&request.tax_relief,"本期减免税额")?})).transpose()?;
            let result=calculate_pay(gross,deductions,employer,baseline.as_ref().zip(current.as_ref()))?;
            let input=MonthlyRequest{region:region.clone(),employee_id:employee_id.clone(),period:request.period.clone(),payout_month:request.payout_month.clone(),standard_days:standard.to_string(),actual_days:actual.to_string(),performance:performance_input.to_string(),adjustment:adjustment_input.to_string(),employee_deductions:contribution_input.to_string(),employer_contributions:employer_input.to_string(),tax_baseline:baseline,basic_deduction:current.as_ref().map(|v|v.basic_deduction.to_string()),additional_deduction:current.as_ref().map(|v|v.additional_deduction.to_string()),other_deduction:current.as_ref().map(|v|v.other_deduction.to_string()),exempt_income:current.as_ref().map(|v|v.exempt_income.to_string()),tax_relief:current.as_ref().map(|v|v.tax_relief.to_string())};
            let id=Uuid::new_v4().to_string();
            let snapshot=MonthlySnapshot{calculation_origin:"import_batch".into(),applied_rules:selected_rules.clone(),id:id.clone(),subject_id:session.subject.id.clone(),employee_id,period:request.period.clone(),payout_month:request.payout_month.clone(),fixed_salary:fixed,performance:performance_amount,adjustment,result,input,rule_trace:trace};
            records.push((id.clone(),serde_json::to_string(&snapshot).map_err(|e|e.to_string())?));ids.push(id);
        }
        let run=CalculationRun::new(session,&request.period,&request.payout_month,"batch",ids.clone())?;
        let audit_target=serde_json::json!({"run_version":run.version,"employee_count":records.len()}).to_string();
        session.insert_calculation_run(&run.id,&run,&records,"calculate_batch",&audit_target)?;
        Ok(BatchResult{run_id:run.id,run_version:run.version,version_ids:ids,warnings})
}

#[cfg(test)]
mod tests{
    use super::*;
    use payroll_import::{read_file,validate,ImportKind};
    use payroll_storage::Store;
    use std::{fs,path::PathBuf};
    fn fixture(name:&str)->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/mock-payroll/E-A").join(name)}
    #[test]
    fn duplicate_period_rows_are_not_silently_overwritten(){
        let row=ValidatedRow{sheet:"CSV".into(),row:2,fields:BTreeMap::from([("employee_id".into(),"E1".into()),("period".into(),"2026-10".into())])};
        assert!(index(vec![row.clone(),row],"period","2026-10").unwrap_err().contains("重复导入"));
    }
    #[test]
    fn imported_two_months_match_independent_mock_results(){
        let root=std::env::temp_dir().join(format!("payroll-batch-{}",Uuid::new_v4()));
        let store=Store::new(&root).unwrap();
        let subject=store.create_subject("测试甲","1234","上海","test-password").unwrap();
        let mut session=store.unlock(&subject.id,"test-password").unwrap();
        let sources:[(ImportKind,&str,&[(&str,&str)]);6]=[
            (ImportKind::Employees,"employees.csv",&[("subject_id","主体ID"),("employee_id","工号"),("name","姓名"),("base_salary","固定月薪")]),
            (ImportKind::Attendance,"attendance.csv",&[("subject_id","主体ID"),("employee_id","工号"),("period","所属期"),("standard_days","标准工作日"),("actual_days","实出勤日"),("additional_deduction","专项附加扣除")]),
            (ImportKind::Performance,"performance.csv",&[("subject_id","主体ID"),("employee_id","工号"),("period","所属期"),("performance","绩效奖金")]),
            (ImportKind::Adjustments,"adjustments.csv",&[("subject_id","主体ID"),("employee_id","工号"),("period","所属期"),("adjustment","调整项")]),
            (ImportKind::Social,"social.csv",&[("subject_id","主体ID"),("employee_id","工号"),("period","所属期"),("employee_contribution","员工承担"),("employer_contribution","单位承担")]),
            (ImportKind::TaxHistory,"tax_baseline.csv",&[("subject_id","主体ID"),("employee_id","工号"),("payout_month","计划发放月"),("cumulative_income","此前累计收入"),("cumulative_exempt_income","此前累计免税收入"),("cumulative_basic_deduction","此前累计基本减除费用"),("cumulative_special_deduction","此前累计专项扣除"),("cumulative_additional_deduction","此前累计专项附加扣除"),("cumulative_other_deduction","此前累计其他扣除"),("cumulative_tax_relief","此前累计减免税额"),("cumulative_withheld_tax","此前实际已预扣个税")]),
        ];
        for (kind,filename,columns) in sources{
            let source=String::from_utf8(fs::read(fixture(filename)).unwrap()).unwrap().replace("E-A",&subject.id);
            let file=root.join(format!("test-{filename}"));fs::write(&file,source.as_bytes()).unwrap();
            let preview=read_file(&file).unwrap();
            let mapping=columns.iter().map(|(key,label)|((*key).into(),(*label).into())).collect();
            let validated=validate(&preview,kind,&subject.id,"CSV",&mapping).unwrap();
            assert!(validated.errors.is_empty(),"{}: {:?}",filename,validated.errors);
            let id=Uuid::new_v4().to_string();
            let rows=validated.rows.iter().map(|r|(r.row,serde_json::to_string(r).unwrap())).collect::<Vec<_>>();
            let employees=if kind==ImportKind::Employees{validated.rows.iter().map(|r|{
                let e=Employee{id:r.fields["employee_id"].clone(),name:r.fields["name"].clone(),base_salary:r.fields["base_salary"].clone(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:String::new(),leave_date:String::new(),effective_from:String::new()};
                (e.id.clone(),serde_json::to_string(&e).unwrap())
            }).collect()}else{Vec::new()};
            let metadata=crate::app_commands::ImportBatch{id:id.clone(),subject_id:subject.id.clone(),filename:filename.into(),sha256:preview.sha256.clone(),sheet:"CSV".into(),kind:format!("{kind:?}"),row_count:rows.len(),withdrawn_at:None};
            session.import_batch(&id,filename,&preview.sha256,source.as_bytes(),&metadata,&rows,&employees).unwrap();
        }
        let before=crate::work_progress::progress(&session,&crate::work_progress::WorkScope{period:"2026-10".into(),payout_month:"2026-11".into(),region:"上海".into()}).unwrap();assert_eq!(before.steps[0].state,"done");assert_eq!(before.steps[1].state,"done");assert_eq!(before.steps[2].state,"pending");
        let mut expected=BTreeMap::new();
        let mut reader=csv::Reader::from_path(fixture("expected_payroll.csv")).unwrap();
        for record in reader.deserialize::<BTreeMap<String,String>>(){let row=record.unwrap();expected.insert((row["工号"].clone(),row["计划发放月"].clone()),row);}
        for (period,payout) in [("2026-10","2026-11"),("2026-11","2026-12")] {
            let result=run(&mut session,BatchRequest{period:period.into(),payout_month:payout.into(),region:"上海".into(),basic_deduction:"5000".into(),other_deduction:"0".into(),exempt_income:"0".into(),tax_relief:"0".into()}).unwrap();
            assert_eq!(result.run_version,1);
            assert_eq!(result.version_ids.len(),2);assert!(result.warnings.is_empty(),"{:?}",result.warnings);
            assert_eq!(crate::export_runs::selected_snapshots(&session,&result.run_id).unwrap().1.len(),2);
            for id in result.version_ids{
                let snapshot:MonthlySnapshot=session.get(Collection::Calculations,&id).unwrap().unwrap();
                let target=&expected[&(snapshot.employee_id.clone(),payout.into())];
                assert_eq!(snapshot.result.gross,amount(&target["应发工资"],"expected").unwrap());
                assert_eq!(snapshot.result.withholding.unwrap().current_withholding,amount(&target["本期预扣个税"],"expected").unwrap());
                assert_eq!(snapshot.result.net,Some(amount(&target["实发工资"],"expected").unwrap()));
            }
        }
        let scope=crate::work_progress::WorkScope{period:"2026-10".into(),payout_month:"2026-11".into(),region:"上海".into()};
        let progress=crate::work_progress::progress(&session,&scope).unwrap();assert_eq!((progress.calculated_count,progress.default_rule_count,progress.custom_rule_count,progress.stale_count),(2,9,0,0));assert!(progress.steps.iter().take(4).all(|step|step.state=="done"));assert_eq!(progress.steps[4].state,"pending");
        let future=crate::work_progress::WorkScope{period:"2027-01".into(),payout_month:"2027-02".into(),region:"上海".into()};assert_eq!(crate::work_progress::progress(&session,&future).unwrap().calculated_count,0);
        session.log_action("export_payroll","2 versions").unwrap();assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().exported_count,0);
        let export_ids:Vec<String>=session.page::<MonthlySnapshot>(Collection::Calculations,1,20).unwrap().records.into_iter().filter(|value|value.period==scope.period).map(|value|value.id).collect();
        session.log_action("export_payroll",&serde_json::json!({"version_ids":export_ids}).to_string()).unwrap();assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().steps[4].state,"done");
        let attendance_batch=session.page::<crate::app_commands::ImportBatch>(Collection::ImportBatches,1,20).unwrap().records.into_iter().find(|value|value.filename=="attendance.csv").unwrap();session.set_import_withdrawn(&attendance_batch.id,true).unwrap();assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().stale_count,2);session.set_import_withdrawn(&attendance_batch.id,false).unwrap();assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().stale_count,0);
        let old=session.page::<MonthlySnapshot>(Collection::Calculations,1,20).unwrap().records;
        let custom=payroll_core::rules::Rule{code:"fixed_salary".into(),label:"自定义计薪".into(),expression:"base_salary * 2".into(),valid_from:"2026-10-01".into(),valid_to:None,region:Some("上海".into()),source:"测试制度".into(),version:1,status:payroll_core::rules::RuleStatus::Active};
        session.insert_immutable(Collection::Rules,"fixed_salary-1",&custom).unwrap();
        assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().stale_count,2);
        let recalculated=run(&mut session,BatchRequest{period:"2026-10".into(),payout_month:"2026-11".into(),region:"上海".into(),basic_deduction:"5000".into(),other_deduction:"0".into(),exempt_income:"0".into(),tax_relief:"0".into()}).unwrap();
        assert_eq!(recalculated.run_version,2);
        for id in recalculated.version_ids{let value:MonthlySnapshot=session.get(Collection::Calculations,&id).unwrap().unwrap();assert_eq!(value.applied_rules.iter().find(|rule|rule.code=="fixed_salary").unwrap().version,1);assert_eq!(value.result.gross,if value.employee_id=="A-1001"{Decimal::from(21000)}else{Decimal::from(25000)});assert_eq!(value.input.performance,"1000");}
        assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().stale_count,0);assert_eq!(crate::work_progress::progress(&session,&scope).unwrap().exported_count,0);
        for snapshot in old{let original:MonthlySnapshot=session.get(Collection::Calculations,&snapshot.id).unwrap().unwrap();assert_eq!(original.result.gross,snapshot.result.gross);assert!(original.applied_rules.iter().all(|rule|rule.version==0));}
        drop(session);fs::remove_dir_all(root).unwrap();
    }
}
