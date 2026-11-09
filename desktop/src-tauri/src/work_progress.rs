use crate::{app_commands::{AppState,AuditEntry,Employee,MonthlySnapshot},import_view::{data_rows,DataFilter,DataRow},payroll_rules};
use crate::export_runs::CalculationRun;
use payroll_core::{Decimal,rules::Rule};
use payroll_storage::{Collection,Session};
use serde::{Deserialize,Serialize};
use std::collections::{BTreeMap,BTreeSet};
use tauri::State;
#[derive(Clone,Deserialize)]pub struct WorkScope{pub period:String,pub payout_month:String,pub region:String}
#[derive(Serialize)]pub struct WorkIssue{pub message:String,pub target:String}
#[derive(Serialize)]pub struct WorkStep{pub label:String,pub state:String,pub detail:String,pub target:String}
#[derive(Serialize)]pub struct WorkProgress{pub employee_count:usize,pub import_batch_count:usize,pub default_rule_count:usize,pub custom_rule_count:usize,pub calculated_count:usize,pub version_count:usize,pub stale_count:usize,pub anomaly_count:usize,pub exported_count:usize,pub steps:Vec<WorkStep>,pub issues:Vec<WorkIssue>}
fn all<T:for<'de>Deserialize<'de>>(session:&Session,collection:Collection)->Result<Vec<T>,String>{let mut rows=Vec::new();let mut page=1;loop{let data=session.page::<T>(collection,page,100)?;rows.extend(data.records);if page as u64*100>=data.total{break}page+=1}Ok(rows)}
fn same_number(fields:&BTreeMap<String,String>,key:&str,value:&str)->bool{fields.get(key).and_then(|text|text.parse::<Decimal>().ok()).zip(value.parse::<Decimal>().ok()).is_some_and(|(a,b)|a==b)}
fn current_snapshot(session:&Session,snapshot:&MonthlySnapshot,rows:&[DataRow],rules:&[Rule],region:&str,configurations:&[payroll_core::contributions::Configuration])->Result<bool,String>{
 if snapshot.calculation_origin!="manual"{
 let find=|kind:&str|{let found:Vec<_>=rows.iter().filter(|row|row.kind==kind&&row.fields.get("employee_id")==Some(&snapshot.employee_id)).collect();if found.len()==1{Some(found[0])}else{None}};
 let Some(att)=find("Attendance") else{return Ok(false)};let Some(perf)=find("Performance") else{return Ok(false)};let Some(adj)=find("Adjustments") else{return Ok(false)};
 if !same_number(&att.fields,"standard_days",&snapshot.input.standard_days)||!same_number(&att.fields,"actual_days",&snapshot.input.actual_days)||!same_number(&perf.fields,"performance",&snapshot.input.performance)||!same_number(&adj.fields,"adjustment",&snapshot.input.adjustment){return Ok(false)}
 let baseline=find("TaxHistory").and_then(|row|crate::batch::tax_baseline(&payroll_import::ValidatedRow{sheet:row.sheet.clone(),row:row.row,fields:row.fields.clone()}));
 if serde_json::to_value(baseline).map_err(|e|e.to_string())?!=serde_json::to_value(&snapshot.input.tax_baseline).map_err(|e|e.to_string())?{return Ok(false)}
 let additional=att.fields.get("additional_deduction").and_then(|v|v.parse::<Decimal>().ok());if additional!=snapshot.input.additional_deduction.as_deref().and_then(|v|v.parse::<Decimal>().ok())&&snapshot.input.tax_baseline.is_some(){return Ok(false)}
 }
 if let Some(saved)=&snapshot.contributions{match payroll_core::contributions::resolve(configurations,&snapshot.employee_id,&snapshot.period){Ok(current) if &current==saved=>{},_=>return Ok(false)}}else{return Ok(false)}
 if !snapshot.applied_rules.is_empty()&&serde_json::to_value(rules).map_err(|e|e.to_string())?!=serde_json::to_value(&snapshot.applied_rules).map_err(|e|e.to_string())?{return Ok(false)}
 let Some(employee)=session.get::<Employee>(Collection::Employees,&snapshot.employee_id)? else{return Ok(false)};
 let plan=match crate::batch::plan_for(session,&employee,&snapshot.period){Ok(value)=>value,Err(_)=>return Ok(false)};
 if let Some(saved)=&snapshot.salary_plan{if saved.id!=plan.id||saved.base_salary.parse::<Decimal>().ok()!=plan.base_salary.parse::<Decimal>().ok()||saved.effective_from.get(..7)!=plan.effective_from.get(..7)||saved.effective_to.as_deref().and_then(|v|v.get(..7))!=plan.effective_to.as_deref().and_then(|v|v.get(..7)){return Ok(false)}}
 let base=plan.base_salary.parse::<Decimal>().map_err(|_|"工资金额无效")?;
 let parsed=|value:&str|value.parse::<Decimal>().map_err(|_|"快照输入金额无效".to_string());
 let fields=BTreeMap::from([("base_salary".into(),base),("standard_days".into(),parsed(&snapshot.input.standard_days)?),("actual_days".into(),parsed(&snapshot.input.actual_days)?),("performance".into(),parsed(&snapshot.input.performance)?),("adjustment".into(),parsed(&snapshot.input.adjustment)?),("employee_contribution".into(),parsed(&snapshot.input.employee_deductions)?),("employer_contribution".into(),parsed(&snapshot.input.employer_contributions)?)]);
 let current=match payroll_rules::calculate(rules,&fields,&format!("{}-01",snapshot.period),region){Ok(value)=>value,Err(_)=>return Ok(false)};
 Ok(current.fixed==snapshot.fixed_salary&&current.gross==snapshot.result.gross&&current.deductions==snapshot.result.employee_deductions)
}
pub fn progress(session:&Session,scope:&WorkScope)->Result<WorkProgress,String>{
 let rows=data_rows(session,&DataFilter{period:scope.period.clone(),payout_month:scope.payout_month.clone(),year:scope.period.get(..4).ok_or("所属期格式错误")?.into(),employee_id:String::new(),batch_id:String::new()})?;
 let relevant:Vec<_>=rows.into_iter().filter(|row|!["Bonus","HistoricalPayroll"].contains(&row.kind.as_str())).collect();
 let region=if scope.region.is_empty(){session.subject.region.as_str()}else{scope.region.as_str()};let date=format!("{}-01",scope.period);let effective=payroll_rules::selected(&payroll_rules::all(session)?,&date,region);
 let rule_error=payroll_rules::validate_configured(&effective).err();let configurations=crate::contribution_config::all(session)?;
 let mut issues=Vec::new();let mut input_errors=0;let mut tax_missing=0;let mut default_configs=0;let mut employee_configs=0;let mut missing_configs=0;
 let attendance_ids:BTreeSet<String>=relevant.iter().filter(|row|row.kind=="Attendance").filter_map(|row|row.fields.get("employee_id").cloned()).collect();
 for id in &attendance_ids{
  if session.get::<Employee>(Collection::Employees,id)?.is_none(){input_errors+=1;issues.push(WorkIssue{message:format!("{id} 缺少员工档案"),target:"employees".into()});}
  if let Some(employee)=session.get::<Employee>(Collection::Employees,id)?{if let Err(message)=crate::batch::salary_for(session,&employee,&scope.period){input_errors+=1;issues.push(WorkIssue{message,target:"employees".into()});}}
  match payroll_core::contributions::resolve(&configurations,id,&scope.period){Err(message)=>{input_errors+=1;missing_configs+=1;issues.push(WorkIssue{message,target:"employees".into()});},Ok(rows)=>{if rows.iter().any(|r|[&r.base,&r.employee_percent,&r.employer_percent].iter().any(|f|f.scope=="employee")){employee_configs+=1}else{default_configs+=1}}}
  for kind in ["Attendance","Performance","Adjustments"]{let count=relevant.iter().filter(|row|row.kind==kind&&row.fields.get("employee_id")==Some(id)).count();if count!=1{input_errors+=1;issues.push(WorkIssue{message:format!("{id} 的 {} 输入{}",match kind{"Attendance"=>"考勤","Performance"=>"绩效","Adjustments"=>"补发补扣",_=>"社保公积金"},if count==0{"缺失"}else{"重复"}),target:"imports".into()});}}
  let tax:Vec<_>=relevant.iter().filter(|row|row.kind=="TaxHistory"&&row.fields.get("employee_id")==Some(id)).collect();let valid_tax=tax.len()==1&&crate::batch::tax_baseline(&payroll_import::ValidatedRow{sheet:tax[0].sheet.clone(),row:tax[0].row,fields:tax[0].fields.clone()}).is_some();let additional=relevant.iter().find(|row|row.kind=="Attendance"&&row.fields.get("employee_id")==Some(id)).and_then(|row|row.fields.get("additional_deduction")).and_then(|value|value.parse::<Decimal>().ok());
  if !valid_tax||additional.is_none(){tax_missing+=1;issues.push(WorkIssue{message:format!("{id} 税务基线或专项附加扣除不完整，个税及实发不可准确测算"),target:"imports".into()});}
 }
 if let Some(message)=&rule_error{issues.push(WorkIssue{message:format!("当期规则不可用：{message}"),target:"rules".into()});}
 let versions:Vec<MonthlySnapshot>=all::<MonthlySnapshot>(session,Collection::Calculations)?.into_iter().filter(|value|value.period==scope.period&&value.payout_month==scope.payout_month).collect();let mut latest=BTreeMap::new();for value in &versions{latest.entry(value.employee_id.clone()).or_insert(value);}
 let manual_count=latest.values().filter(|value|value.calculation_origin=="manual").count();
 if attendance_ids.is_empty()&&manual_count==0{issues.push(WorkIssue{message:"所选所属期尚无考勤名单，也没有已记录的手动试算输入".into(),target:"imports".into()});}
 let mut stale_count=0;let mut anomaly_count=0;
 for value in latest.values(){if rule_error.is_some()||!current_snapshot(session,value,&relevant,&effective,region,&configurations)?{stale_count+=1;}if value.result.withholding.is_none()||value.result.net.is_some_and(|net|net.is_sign_negative()){anomaly_count+=1;issues.push(WorkIssue{message:format!("{} 的最新计算{}",value.employee_id,if value.result.withholding.is_none(){"缺少税额及实发测算"}else{"预估实发为负"}),target:"history".into()});}}
 if stale_count>0{issues.push(WorkIssue{message:format!("{stale_count} 人的当前输入或规则与最新计算快照不一致，请重新试算"),target:"payroll".into()});}
 let all_ids:BTreeSet<&str>=latest.values().map(|value|value.id.as_str()).collect();let mut exported=BTreeSet::new();for entry in all::<AuditEntry>(session,Collection::Audit)?{if ["export_payroll","export_payroll_xlsx"].contains(&entry.action.as_str()){if let Ok(value)=serde_json::from_str::<serde_json::Value>(&entry.target){if let Some(ids)=value["version_ids"].as_array(){for id in ids.iter().filter_map(serde_json::Value::as_str){if all_ids.contains(id){exported.insert(id.to_owned());}}}}}}
 let batch_ids:BTreeSet<_>=relevant.iter().map(|row|&row.batch_id).collect();let calculated=latest.len();let missing=attendance_ids.iter().filter(|id|!latest.contains_key(*id)).count();
 let step=|label:&str,state:&str,detail:String,target:&str|WorkStep{label:label.into(),state:state.into(),detail,target:target.into()};
 let steps=vec![
 step("导入与校验",if attendance_ids.is_empty()&&manual_count==0{"pending"}else if input_errors+tax_missing>0{"warning"}else{"done"},if attendance_ids.is_empty(){if manual_count>0{format!("使用 {manual_count} 人的已验证手动输入")}else{"尚无当期考勤输入".into()}}else{format!("考勤范围 {} 人 · {} 项输入问题 · {} 人税务资料待补；缴费采用主体默认 {default_configs} 人 / 员工覆盖 {employee_configs} 人 / 缺失 {missing_configs} 人",attendance_ids.len(),input_errors,tax_missing)},"imports"),
 step("匹配规则",if rule_error.is_some(){"warning"}else{"done"},format!("{} 项默认 · {} 项自定义生效",effective.iter().filter(|rule|rule.version==0).count()+3,effective.iter().filter(|rule|rule.version>0).count()),"rules"),
 step("工资试算",if calculated==0{"pending"}else if stale_count>0||missing>0{"warning"}else{"done"},format!("{calculated} 人已有结果 · {missing} 人未算 · {stale_count} 人需重算"),"payroll"),
 step("异常检查",if calculated==0{"pending"}else if anomaly_count>0||stale_count>0||missing>0||input_errors>0||tax_missing>0{"warning"}else{"done"},if calculated==0{"试算后检查结果异常".into()}else{format!("最新结果 {anomaly_count} 人有异常；人工复核仍需查看明细")},"history"),
 step("导出",if calculated==0{"pending"}else if exported.len()==calculated&&stale_count==0&&missing==0{"done"}else if !exported.is_empty(){"warning"}else{"pending"},format!("最新计算结果已导出 {} / {calculated} 人",exported.len()),"exports")];
 let run_count=all::<CalculationRun>(session,Collection::CalculationRuns)?.into_iter().filter(|run|run.period==scope.period&&run.payout_month==scope.payout_month).count();
 Ok(WorkProgress{employee_count:attendance_ids.len().max(manual_count),import_batch_count:batch_ids.len(),default_rule_count:effective.iter().filter(|rule|rule.version==0).count()+3,custom_rule_count:effective.iter().filter(|rule|rule.version>0).count(),calculated_count:calculated,version_count:run_count,stale_count,anomaly_count,exported_count:exported.len(),steps,issues})
}
#[tauri::command]pub fn workspace_progress(state:State<AppState>,scope:WorkScope)->Result<WorkProgress,String>{state.with_session(|session|progress(session,&scope))}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn manual_calculation_is_not_mistaken_for_missing_import_work(){
  let root=std::env::temp_dir().join(format!("manual-progress-{}",uuid::Uuid::new_v4()));let store=payroll_storage::Store::new(root.clone()).unwrap();let subject=store.create_subject("手动主体","1234","上海","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
  let employee=Employee{id:"M1".into(),name:"手动员工".into(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"10000".into(),effective_from:"2026-01-01".into()};session.upsert(Collection::Employees,&employee.id,&employee).unwrap();
  let input=crate::app_commands::MonthlyRequest{region:"上海".into(),employee_id:employee.id.clone(),period:"2026-10".into(),payout_month:"2026-11".into(),standard_days:"22".into(),actual_days:"22".into(),performance:"0".into(),adjustment:"0".into(),employee_deductions:"800".into(),employer_contributions:"1200".into(),tax_baseline:None,basic_deduction:None,additional_deduction:None,other_deduction:None,exempt_income:None,tax_relief:None};
  let rules=payroll_rules::selected(&[],"2026-10-01","上海");let fields=BTreeMap::from([("base_salary".into(),Decimal::from(10000)),("standard_days".into(),Decimal::from(22)),("actual_days".into(),Decimal::from(22)),("performance".into(),Decimal::ZERO),("adjustment".into(),Decimal::ZERO),("employee_contribution".into(),Decimal::from(800)),("employer_contribution".into(),Decimal::from(1200))]);let values=payroll_rules::calculate(&rules,&fields,"2026-10-01","上海").unwrap();
  let result=payroll_core::monthly::calculate_pay(values.gross,values.deductions,values.employer,None).unwrap();let snapshot=MonthlySnapshot{needs_recalculation:false,salary_plan:None,contributions:None,contribution_configurations:Vec::new(),calculation_origin:"manual".into(),applied_rules:rules,id:"manual-1".into(),subject_id:subject.id.clone(),employee_id:employee.id.clone(),period:input.period.clone(),payout_month:input.payout_month.clone(),fixed_salary:values.fixed,performance:values.performance,adjustment:values.adjustment,result,input,rule_trace:values.trace};session.insert_immutable(Collection::Calculations,&snapshot.id,&snapshot).unwrap();
  let scope=WorkScope{period:"2026-10".into(),payout_month:"2026-11".into(),region:"上海".into()};let result=progress(&session,&scope).unwrap();assert_eq!(result.steps[0].state,"done");assert_eq!(result.calculated_count,1);assert_eq!(result.stale_count,1);assert_eq!(result.anomaly_count,1);assert_eq!(result.steps[3].state,"warning");assert!(progress(&session,&WorkScope{period:"x".into(),..scope}).is_err());drop(session);let _=std::fs::remove_dir_all(root);
 }
}

/// Derive freshness for returned views; never rewrite stored calculation snapshots.
pub fn mark_recalculation(session:&Session,snapshots:&mut[MonthlySnapshot])->Result<(),String>{
 let configurations=crate::contribution_config::all(session)?;let configured_rules=payroll_rules::all(session)?;
 for snapshot in snapshots{let region=if snapshot.input.region.is_empty(){session.subject.region.as_str()}else{snapshot.input.region.as_str()};let rules=payroll_rules::selected(&configured_rules,&format!("{}-01",snapshot.period),region);
 let rows=data_rows(session,&DataFilter{period:snapshot.period.clone(),payout_month:snapshot.payout_month.clone(),year:snapshot.period.get(..4).unwrap_or("").into(),employee_id:snapshot.employee_id.clone(),batch_id:String::new()})?;
 snapshot.needs_recalculation=!current_snapshot(session,snapshot,&rows,&rules,region,&configurations)?;
 }Ok(())
}
