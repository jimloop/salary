use crate::app_commands::{amount,AppState,SalaryPlan};
use payroll_import::{ImportKind,RowError,ValidatedRow,Validation};
use payroll_storage::{Collection,Session};
use tauri::State;

pub fn row_plan(row:&ValidatedRow)->Result<SalaryPlan,String>{
 let get=|key:&str|row.fields.get(key).cloned().unwrap_or_default();
 let employee_id=get("employee_id");let effective_from=get("effective_from");
 Ok(SalaryPlan{id:format!("import:{employee_id}:{effective_from}"),employee_id,effective_from,effective_to:row.fields.get("effective_to").filter(|v|!v.is_empty()).cloned(),base_salary:amount(&get("base_salary"),"固定月薪")?.to_string(),note:get("note")})
}
pub fn plans(session:&Session)->Result<Vec<SalaryPlan>,String>{
 let mut result=Vec::new();let mut page=1;loop{let batch=session.page::<SalaryPlan>(Collection::SalaryPlans,page,100)?;result.extend(batch.records);if page as u64*100>=batch.total{break}page+=1;}
 for row in session.imported_rows("SalaryPlans")?{result.push(row_plan(&serde_json::from_value(row).map_err(|e|e.to_string())?)?);}
 Ok(result)
}
pub fn overlap(a:&SalaryPlan,b:&SalaryPlan)->bool{a.employee_id==b.employee_id&&a.effective_from.as_str()<=b.effective_to.as_deref().unwrap_or("9999-12-31")&&b.effective_from.as_str()<=a.effective_to.as_deref().unwrap_or("9999-12-31")}
pub fn validate_plans(session:&Session,kind:ImportKind,validation:&mut Validation)->Result<(),String>{
 if kind!=ImportKind::SalaryPlans{return Ok(())}let mut existing=plans(session)?;
 for row in &validation.rows{if let Ok(plan)=row_plan(row){if existing.iter().any(|old|overlap(old,&plan)){validation.errors.push(RowError{sheet:row.sheet.clone(),row:row.row,field:"effective_from".into(),reason:"与已有方案或本文件其他方案的有效期重叠；请调整期间或撤销重复批次".into()});}existing.push(plan);}}
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
   if kind==ImportKind::SalaryPlans{validate_plans(&session,kind,&mut validation).unwrap();assert_eq!(validation.errors.len(),1);}
  }
  assert_eq!(crate::batch::salary_for(&session,&employee,"2026-10").unwrap().to_string(),"12000");assert_eq!(bonus_rows(&session,"2026").unwrap()[0].fields["target"],"20000");
  session.set_import_withdrawn("plan",true).unwrap();session.set_import_withdrawn("bonus",true).unwrap();assert_eq!(crate::batch::salary_for(&session,&employee,"2026-10").unwrap().to_string(),"10000");assert!(bonus_rows(&session,"2026").unwrap().is_empty());
  session.set_import_withdrawn("plan",false).unwrap();session.set_import_withdrawn("bonus",false).unwrap();assert_eq!(plans(&session).unwrap().len(),1);assert_eq!(bonus_rows(&session,"2026").unwrap().len(),1);
  drop(session);let _=std::fs::remove_dir_all(root);
 }
 #[test]fn bonus_rejects_impossible_days_and_negative_values(){
  let root=std::env::temp_dir().join(format!("bonus-validation-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let path=root.join("invalid.csv");std::fs::write(&path,"subject_id,employee_id,bonus_year,target,performance_factor,eligible_days,plan_days,minimum,maximum\nA,E1,2026,-100,1.2,400,365,500,100\n").unwrap();let preview=read_file(&path).unwrap();let mapping=preview.sheets[0].headers.iter().map(|key|(key.clone(),key.clone())).collect();let result=validate(&preview,ImportKind::Bonus,"A","CSV",&mapping).unwrap();assert!(result.errors.iter().any(|error|error.field=="target"));assert!(result.errors.iter().any(|error|error.field=="eligible_days"));assert!(result.errors.iter().any(|error|error.field=="maximum"));let _=std::fs::remove_dir_all(root);
 }
}
