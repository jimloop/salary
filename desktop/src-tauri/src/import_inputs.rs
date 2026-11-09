use crate::app_commands::{amount,AppState,SalaryPlan};
use payroll_import::{ImportKind,RowError,ValidatedRow,Validation};
use payroll_storage::{Collection,Session};
use tauri::State;

pub fn plan_month(value:&str,end:bool)->Result<String,String>{
 if payroll_core::contributions::valid_month(value){return Ok(value.into());}
 let date=chrono::NaiveDate::parse_from_str(value,"%Y-%m-%d").map_err(|_|"工资方案月份无效")?;
 use chrono::Datelike;
 let boundary=if end{date.succ_opt().is_some_and(|next|next.day()==1)}else{date.day()==1};
 if !boundary{return Err(format!("旧工资方案 {value} 为月中日期，请明确调整有效月份"));}Ok(date.format("%Y-%m").to_string())
}
pub fn row_plan(row:&ValidatedRow)->Result<SalaryPlan,String>{
 let get=|key:&str|row.fields.get(key).cloned().unwrap_or_default();
 let employee_id=get("employee_id");let effective_from=get("effective_from");
 Ok(SalaryPlan{id:format!("import:{employee_id}:{effective_from}"),employee_id,effective_from,effective_to:row.fields.get("effective_to").filter(|v|!v.is_empty()).cloned(),base_salary:amount(&get("base_salary"),"固定月薪")?.to_string(),note:get("note")})
}
pub fn plans(session:&Session)->Result<Vec<SalaryPlan>,String>{
 let mut stored=std::collections::BTreeMap::new();let mut page=1;loop{let batch=session.page::<SalaryPlan>(Collection::SalaryPlans,page,100)?;for plan in batch.records{stored.insert(plan.id.clone(),plan);}if page as u64*100>=batch.total{break}page+=1;}
 let mut result:Vec<_>=stored.values().filter(|p|!p.id.starts_with("import:")).cloned().collect();
 for value in session.imported_data(false)?{let Some(batch_id)=value["batch_id"].as_str()else{continue};let Some(batch)=session.get::<crate::app_commands::ImportBatch>(Collection::ImportBatches,batch_id)?else{continue};if batch.kind!="SalaryPlans"{continue;}let row:ValidatedRow=serde_json::from_value(value["data"].clone()).map_err(|e|e.to_string())?;let mut plan=row_plan(&row)?;plan.id=format!("import:{batch_id}:{}",row.row);result.push(stored.get(&plan.id).cloned().unwrap_or(plan));}
 Ok(result)
}
pub fn validate_timeline(plans:&[SalaryPlan])->Result<(),String>{
 let mut groups=std::collections::BTreeMap::<&str,Vec<(String,Option<String>)>>::new();
 for plan in plans{let start=plan_month(&plan.effective_from,false)?;let end=plan.effective_to.as_deref().map(|v|plan_month(v,true)).transpose()?;if end.as_ref().is_some_and(|end|end<&start){return Err("工资方案截止月份早于生效月份".into());}groups.entry(&plan.employee_id).or_default().push((start,end));}
 for (employee,mut periods) in groups{periods.sort();for pair in periods.windows(2){let Some(end)=&pair[0].1 else{return Err(format!("工号 {employee} 工资方案月份重叠"));};if end>=&pair[1].0{return Err(format!("工号 {employee} 工资方案月份重叠"));}let next=chrono::NaiveDate::parse_from_str(&format!("{end}-01"),"%Y-%m-%d").ok().and_then(|v|v.checked_add_months(chrono::Months::new(1))).map(|v|v.format("%Y-%m").to_string());if next.as_deref()!=Some(pair[1].0.as_str()){return Err(format!("工号 {employee} 工资方案在 {end} 与 {} 之间存在缺失月份，请衔接期间",pair[1].0));}}}Ok(())
}
pub fn overlap(a:&SalaryPlan,b:&SalaryPlan)->bool{a.employee_id==b.employee_id&&a.effective_from.get(..7).unwrap_or(&a.effective_from)<=b.effective_to.as_deref().and_then(|v|v.get(..7)).unwrap_or("9999-12")&&b.effective_from.get(..7).unwrap_or(&b.effective_from)<=a.effective_to.as_deref().and_then(|v|v.get(..7)).unwrap_or("9999-12")}
pub fn validate_plans(session:&Session,kind:ImportKind,validation:&mut Validation)->Result<(),String>{
 if kind!=ImportKind::SalaryPlans{return Ok(())}let mut existing=plans(session)?;
 for row in &validation.rows{if let Ok(plan)=row_plan(row){if let Err(reason)=plan_month(&plan.effective_from,false).and_then(|_|plan.effective_to.as_deref().map(|v|plan_month(v,true)).transpose()){validation.errors.push(RowError{sheet:row.sheet.clone(),row:row.row,field:"effective_from".into(),reason});}if existing.iter().any(|old|overlap(old,&plan)){validation.errors.push(RowError{sheet:row.sheet.clone(),row:row.row,field:"effective_from".into(),reason:"与已有方案或本文件其他方案的有效期重叠；请调整期间或撤销重复批次".into()});}existing.push(plan);}}
 if let Err(reason)=validate_timeline(&existing){validation.errors.push(RowError{sheet:"工资方案".into(),row:0,field:"effective_from".into(),reason});}
 Ok(())
}
#[tauri::command]
pub fn bonus_inputs(state:State<AppState>,year:String)->Result<Vec<ValidatedRow>,String>{state.with_session(|session|bonus_rows(session,&year))}
pub fn bonus_rows(session:&Session,year:&str)->Result<Vec<ValidatedRow>,String>{
 let mut seen=std::collections::BTreeSet::new();let mut result=Vec::new();for value in session.imported_rows("Bonus")?{let row:ValidatedRow=serde_json::from_value(value).map_err(|e|e.to_string())?;if row.fields.get("bonus_year").is_some_and(|value|value==year){let id=row.fields.get("employee_id").ok_or("资格表缺少工号")?;if !seen.insert(id.clone()){return Err(format!("工号 {id} 在 {year} 有重复奖金资格记录，请撤销重复批次"));}result.push(row);}}Ok(result)
}

#[cfg(test)]
mod tests{
 use super::*;use crate::app_commands::{Employee,ImportBatch};use payroll_import::{read_file,validate};use payroll_storage::Store;use std::collections::BTreeMap;
 #[test]fn imported_plans_and_bonus_follow_withdrawal_and_conflicts(){
  let root=std::env::temp_dir().join(format!("salary-inputs-{}",uuid::Uuid::new_v4()));let store=Store::new(root.clone()).unwrap();let subject=store.create_subject("测试主体","1234","上海","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
  let employee=Employee{id:"E1".into(),name:"测试员工".into(),department:"研发".into(),cost_center:"C1".into(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"10000".into(),effective_from:"2026-01-01".into()};session.upsert(Collection::Employees,"E1",&employee).unwrap();
  for (kind,id,source) in [(ImportKind::SalaryPlans,"plan",format!("subject_id,employee_id,effective_from,effective_to,base_salary\n{},E1,2026-01-01,2026-12-31,12000\n",subject.id)),(ImportKind::Bonus,"bonus",format!("subject_id,employee_id,bonus_year,target,performance_factor,eligible_days,plan_days\n{},E1,2026,20000,1.2,180,360\n",subject.id))]{
   let path=root.join(format!("{id}.csv"));std::fs::write(&path,&source).unwrap();let file=read_file(&path).unwrap();let mapping:BTreeMap<String,String>=file.sheets[0].headers.iter().map(|key|(key.clone(),key.clone())).collect();let mut validation=validate(&file,kind,&subject.id,"CSV",&mapping).unwrap();validate_plans(&session,kind,&mut validation).unwrap();assert!(validation.errors.is_empty());
   let rows=validation.rows.iter().map(|row|(row.row,serde_json::to_string(row).unwrap())).collect::<Vec<_>>();let batch=ImportBatch{id:id.into(),subject_id:subject.id.clone(),filename:format!("{id}.csv"),sha256:file.sha256.clone(),sheet:"CSV".into(),kind:format!("{kind:?}"),row_count:1,withdrawn_at:None};session.import_batch(id,&batch.filename,&file.sha256,source.as_bytes(),&batch,&rows,&[]).unwrap();
   if kind==ImportKind::SalaryPlans{validate_plans(&session,kind,&mut validation).unwrap();assert!(!validation.errors.is_empty());}
  }
  assert_eq!(crate::batch::salary_for(&session,&employee,"2026-10").unwrap().to_string(),"12000");assert_eq!(bonus_rows(&session,"2026").unwrap()[0].fields["target"],"20000");
  session.set_import_withdrawn("plan",true).unwrap();session.set_import_withdrawn("bonus",true).unwrap();assert!(crate::batch::salary_for(&session,&employee,"2026-10").is_err());assert!(bonus_rows(&session,"2026").unwrap().is_empty());
  session.set_import_withdrawn("plan",false).unwrap();session.set_import_withdrawn("bonus",false).unwrap();assert_eq!(plans(&session).unwrap().len(),1);assert_eq!(bonus_rows(&session,"2026").unwrap().len(),1);
  drop(session);let _=std::fs::remove_dir_all(root);
 }
 #[test]fn bonus_rejects_impossible_days_and_negative_values(){
  let root=std::env::temp_dir().join(format!("bonus-validation-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let path=root.join("invalid.csv");std::fs::write(&path,"subject_id,employee_id,bonus_year,target,performance_factor,eligible_days,plan_days,minimum,maximum\nA,E1,2026,-100,1.2,400,365,500,100\n").unwrap();let preview=read_file(&path).unwrap();let mapping=preview.sheets[0].headers.iter().map(|key|(key.clone(),key.clone())).collect();let result=validate(&preview,ImportKind::Bonus,"A","CSV",&mapping).unwrap();assert!(result.errors.iter().any(|error|error.field=="target"));assert!(result.errors.iter().any(|error|error.field=="eligible_days"));assert!(result.errors.iter().any(|error|error.field=="maximum"));let _=std::fs::remove_dir_all(root);
 }
}

pub fn migrate_legacy_salaries(session:&mut Session)->Result<(),String>{
    use crate::app_commands::{Employee,ImportBatch};
    let mut known:std::collections::BTreeSet<String>=plans(session)?.into_iter().map(|p|p.employee_id).collect();
    let mut batches=std::collections::BTreeSet::new();let mut page=1;
    loop{let rows=session.page::<ImportBatch>(Collection::ImportBatches,page,100)?;for row in rows.records{if row.kind=="SalaryPlans"{batches.insert(row.id);}}if u64::from(page)*100>=rows.total{break}page+=1;}
    for row in session.imported_data(true)?{if row["batch_id"].as_str().is_some_and(|id|batches.contains(id)){if let Some(id)=row["data"]["fields"]["employee_id"].as_str(){known.insert(id.into());}}}
    let mut records=Vec::new();let mut page=1;
    loop{let employees=session.page::<Employee>(Collection::Employees,page,100)?;for employee in employees.records{
        if known.contains(&employee.id){continue;}
        if let (Ok(start),Ok(base))=(plan_month(&employee.effective_from,false),amount(&employee.base_salary,"旧档案月薪")){if base>=payroll_core::Decimal::ZERO{
            let plan=SalaryPlan{id:format!("legacy:{}",employee.id),employee_id:employee.id,effective_from:start,effective_to:None,base_salary:base.to_string(),note:"从旧员工档案明确的月初生效记录迁移；未推断月中日期".into()};records.push((plan.id.clone(),serde_json::to_string(&plan).map_err(|e|e.to_string())?));
        }}
    }if u64::from(page)*100>=employees.total{break}page+=1;}
    session.migrate_salary_plans_once(&records)
}

#[cfg(test)]
mod migration_tests {
 use super::*;
 #[test]
 fn only_explicit_month_boundaries_migrate_and_migration_never_recreates_expired_plans(){
  let root=std::env::temp_dir().join(format!("salary-migration-{}",uuid::Uuid::new_v4()));let store=payroll_storage::Store::new(&root).unwrap();let subject=store.create_subject("迁移测试","1234","上海市","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
  let mut employee=crate::app_commands::Employee{id:"E1".into(),name:"虚构员工".into(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"10000".into(),effective_from:"2026-01-01".into()};session.upsert(Collection::Employees,&employee.id,&employee).unwrap();employee.id="E2".into();employee.effective_from="2026-01-15".into();session.upsert(Collection::Employees,&employee.id,&employee).unwrap();
  migrate_legacy_salaries(&mut session).unwrap();let mut plans=plans(&session).unwrap();assert_eq!(plans.len(),1);assert_eq!(plans[0].effective_from,"2026-01");assert_eq!(plans[0].employee_id,"E1");plans[0].effective_to=Some("2026-09".into());session.upsert(Collection::SalaryPlans,&plans[0].id,&plans[0]).unwrap();migrate_legacy_salaries(&mut session).unwrap();employee.id="E1".into();assert!(crate::batch::salary_for(&session,&employee,"2026-10").is_err());assert!(plan_month("2026-01-15",false).is_err());assert_eq!(plan_month("2024-02-29",true).unwrap(),"2024-02");assert!(plan_month("2026-02-28",false).is_err());
  drop(session);std::fs::remove_dir_all(root).unwrap();
 }
}
