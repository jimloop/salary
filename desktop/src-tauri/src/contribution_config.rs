use crate::app_commands::{AppState, Employee};
use payroll_core::contributions::{self, Configuration, Contribution};
use payroll_storage::{Collection, Session};
use tauri::State;
use uuid::Uuid;

pub(crate) fn all(session: &Session) -> Result<Vec<Configuration>, String> {
    let mut removed=std::collections::BTreeSet::new();let mut deletion_page=1;loop{let rows=session.page::<serde_json::Value>(Collection::ContributionDeletions,deletion_page,100)?;for row in rows.records{if let Some(id)=row["id"].as_str(){removed.insert(id.to_owned());}}if u64::from(deletion_page)*100>=rows.total{break}deletion_page+=1;}
    let mut records = Vec::new();
    let mut page = 1;
    loop {
        let batch = session.page::<Configuration>(Collection::ContributionConfigurations, page, 100)?;
        let total = batch.total;
        records.extend(batch.records.into_iter().filter(|r:&Configuration|!removed.contains(&r.id)));
        if u64::from(page)*100 >= total { break; }
        page += 1;
    }
    let stored:std::collections::BTreeMap<String,Configuration>=records.iter().map(|r:&Configuration|(r.id.clone(),r.clone())).collect();
    records.retain(|r|!r.id.starts_with("import:"));
    for value in session.imported_data(false)? {
        let Some(batch_id)=value["batch_id"].as_str() else{continue};
        if session.get::<serde_json::Value>(Collection::ImportBatches,batch_id)?.is_none_or(|batch|batch["kind"]!="Social"){continue;}
        let fields = &value["data"]["fields"];
        if fields.get("project").is_none() { continue; }
        let record=row_configuration(&serde_json::from_value(value["data"].clone()).map_err(|e|e.to_string())?,batch_id)?;if !removed.contains(&record.id){records.push(stored.get(&record.id).cloned().unwrap_or(record));}
    }
    Ok(records)
}
#[tauri::command]
pub fn contribution_configurations(state: State<AppState>) -> Result<Vec<Configuration>, String> {
    state.with_session(all)
}
fn save(session: &mut Session, mut configuration: Configuration) -> Result<Configuration, String> {
    if let Some(employee) = &configuration.employee_id {
        if session.get::<Employee>(Collection::Employees, employee)?.is_none() { return Err("当前主体没有该员工".into()); }
    }
    if configuration.id.is_empty() {
        if configuration.employee_id.is_some()&&configuration.base.is_none()&&configuration.employee_percent.is_none()&&configuration.employer_percent.is_none(){return Err("至少修改一项员工基数或比例；未修改时无需保存".into());}
        configuration.id = Uuid::new_v4().to_string();
    }
    if session.get::<serde_json::Value>(Collection::ContributionDeletions,&configuration.id)?.is_some(){return Err("配置已删除，请刷新后重试".into());}
    let mut records = all(session)?;
    if configuration.id.starts_with("import:")&&!records.iter().any(|r|r.id==configuration.id){return Err("导入配置已失效，请刷新后重试".into());}
    if let Some(old) = records.iter().find(|r|r.id==configuration.id) {

        if old.employee_id != configuration.employee_id || old.project != configuration.project { return Err("不能更改配置所属员工或项目".into()); }
    }
    records.retain(|r|r.id!=configuration.id);
    records.push(configuration.clone());
    contributions::validate(&records)?;
    session.upsert(Collection::ContributionConfigurations, &configuration.id, &configuration)?;
    Ok(configuration)
}
#[tauri::command]
pub fn contribution_configuration_save(state: State<AppState>, configuration: Configuration) -> Result<Configuration, String> {
    state.with_session_mut(|session| save(session, configuration))
}
#[tauri::command]
pub fn contribution_preview(state: State<AppState>, employee_id: String, period: String) -> Result<Vec<Contribution>, String> {
    state.with_session(|session| {
        if session.get::<Employee>(Collection::Employees, &employee_id)?.is_none() { return Err("当前主体没有该员工".into()); }
        contributions::resolve(&all(session)?, &employee_id, &period)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use payroll_storage::Store;
    #[test]
    fn configurations_survive_reopen_and_are_isolated_and_immutable() {
        let root = std::env::temp_dir().join(format!("contribution-config-{}", Uuid::new_v4()));
        let store = Store::new(&root).unwrap();
        let subject = store.create_subject("配置测试", "1234", "上海市", "test-password").unwrap();
        let other = store.create_subject("隔离测试", "5678", "上海市", "test-password").unwrap();
        let mut session = store.unlock(&subject.id, "test-password").unwrap();
        let configuration = Configuration { id: String::new(), employee_id: None, project: "住房公积金".into(), start: "2026-01".into(), end: Some("2026-12".into()), enabled: true, base: Some(10000.into()), employee_percent: Some(5.into()), employer_percent: Some(5.into()) };
        let saved = save(&mut session, configuration.clone()).unwrap();
        assert!(save(&mut session, saved.clone()).is_ok());
        assert!(save(&mut session, configuration).is_err());
        let mut missing_employee = saved.clone(); missing_employee.id.clear(); missing_employee.employee_id = Some("missing".into());
        assert!(save(&mut session, missing_employee).is_err());
        drop(session);
        let reopened = store.unlock(&subject.id, "test-password").unwrap();
        assert_eq!(all(&reopened).unwrap(), vec![saved]);
        let isolated = store.unlock(&other.id, "test-password").unwrap();
        assert!(all(&isolated).unwrap().is_empty());
        drop(reopened); drop(isolated);
        std::fs::remove_dir_all(root).unwrap();
    }
}

pub(crate) fn row_configuration(row: &payroll_import::ValidatedRow, batch: &str) -> Result<Configuration,String> {
    let get=|key:&str|row.fields.get(key).cloned().unwrap_or_default();
    let number=|key:&str| -> Result<Option<payroll_core::Decimal>,String> { let value=get(key); if value.is_empty(){Ok(None)}else{Ok(Some(crate::app_commands::amount(&value,key)?))} };
    Ok(Configuration{id:format!("import:{batch}:{}",row.row),employee_id:Some(get("employee_id")),project:get("project"),start:get("start"),end:row.fields.get("end").filter(|v|!v.is_empty()).cloned(),enabled:true,base:number("base")?,employee_percent:number("employee_percent")?,employer_percent:number("employer_percent")?})
}
pub(crate) fn validate_import(session:&Session, validation:&mut payroll_import::Validation)->Result<(),String>{
    let mut records=all(session)?;
    for row in &validation.rows {
        match row_configuration(row,"preview") {
            Ok(record)=>{records.push(record);if let Err(reason)=contributions::validate(&records){validation.errors.push(payroll_import::RowError{sheet:row.sheet.clone(),row:row.row,field:"start".into(),reason});} }
            Err(reason)=>validation.errors.push(payroll_import::RowError{sheet:row.sheet.clone(),row:row.row,field:"base".into(),reason})
        }
    }
    Ok(())
}
#[cfg(test)]
pub(crate) fn calculate(session:&Session, employee:&str, month:&str)->Result<Vec<Contribution>,String>{contributions::resolve(&all(session)?,employee,month)}

pub(crate) fn summary(snapshot:&crate::app_commands::MonthlySnapshot)->String{
    match &snapshot.contributions{None=>"旧版金额结果；无基数比例快照".into(),Some(rows)=>rows.iter().map(|r|format!("{}：基数 {}，员工 {}% / {}，单位 {}% / {}",r.project,r.base.value,r.employee_percent.value,r.employee_amount,r.employer_percent.value,r.employer_amount)).collect::<Vec<_>>().join("；")}
}

#[derive(serde::Serialize)]
pub struct ConfigurationStatus { configured_count:usize, default_count:usize, override_count:usize, missing_count:usize }
#[tauri::command]
pub fn contribution_status(state:State<AppState>,period:String,employee_id:String)->Result<ConfigurationStatus,String>{state.with_session(|session|{
    if !contributions::valid_month(&period){return Err("查看月份无效".into());}
    let records=all(session)?;let mut result=ConfigurationStatus{configured_count:0,default_count:0,override_count:0,missing_count:0};let mut page=1;
    loop{let employees=session.page::<Employee>(Collection::Employees,page,100)?;let total=employees.total;
        for employee in employees.records{if !employee_id.is_empty()&&employee_id!=employee.id{continue;}match contributions::resolve(&records,&employee.id,&period){Ok(rows)=>{result.configured_count+=1;if rows.iter().any(|r|[&r.base,&r.employee_percent,&r.employer_percent].iter().any(|f|f.scope=="employee")){result.override_count+=1}else{result.default_count+=1}},Err(_)=>result.missing_count+=1}}
        if u64::from(page)*100>=total{break}page+=1;
    }Ok(result)
})}
#[tauri::command]
pub fn salary_effective(state:State<AppState>,employee_id:String,period:String)->Result<crate::app_commands::SalaryPlan,String>{state.with_session(|session|{
    let employee:Employee=session.get(Collection::Employees,&employee_id)?.ok_or("当前主体没有该员工")?;
    crate::batch::plan_for(session,&employee,&period)
})}

#[cfg(test)]
mod calculation_tests {
    use super::*;
    use crate::app_commands::{MonthlyRequest,SalaryPlan,calculate_monthly};
    #[test]
    fn calculated_amounts_ignore_request_and_snapshots_survive_configuration_changes(){
        let root=std::env::temp_dir().join(format!("contribution-calculation-{}",Uuid::new_v4()));let store=payroll_storage::Store::new(&root).unwrap();let subject=store.create_subject("虚构配置","1234","上海市","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
        let employee=Employee{id:"E1".into(),name:"测试员工".into(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"99999".into(),effective_from:"2026-01-01".into()};session.upsert(Collection::Employees,&employee.id,&employee).unwrap();
        let plan=SalaryPlan{id:"salary".into(),employee_id:employee.id.clone(),effective_from:"2026-01".into(),effective_to:None,base_salary:"10000".into(),note:"虚构".into()};session.upsert(Collection::SalaryPlans,&plan.id,&plan).unwrap();
        let configuration=Configuration{id:"default".into(),employee_id:None,project:"住房公积金".into(),start:"2026-01".into(),end:Some("2026-10".into()),enabled:true,base:Some(10000.into()),employee_percent:Some(payroll_core::Decimal::new(25,1)),employer_percent:Some(5.into())};save(&mut session,configuration.clone()).unwrap();
        let request:MonthlyRequest=serde_json::from_value(serde_json::json!({"employee_id":"E1","period":"2026-10","payout_month":"2026-11","standard_days":"22","actual_days":"22","performance":"0","adjustment":"0","employee_deductions":"9999","employer_contributions":"9999"})).unwrap();
        let original=calculate_monthly(&mut session,request.clone()).unwrap();assert_eq!(original.result.gross,10000.into());assert_eq!(original.result.employee_deductions,250.into());assert_eq!(original.result.employer_cost,10500.into());assert_eq!(original.input.employee_deductions.parse::<payroll_core::Decimal>().unwrap(),250.into());assert!(original.result.net.is_none());assert_eq!(original.salary_plan.as_ref().unwrap().id,"salary");
        let mut next=configuration;next.id="next".into();next.start="2026-11".into();next.end=None;next.base=Some(12000.into());save(&mut session,next).unwrap();
        let mut next_request=request.clone();next_request.period="2026-11".into();let new=calculate_monthly(&mut session,next_request).unwrap();assert_eq!(new.result.employee_deductions,300.into());
        let unchanged:crate::app_commands::MonthlySnapshot=session.get(Collection::Calculations,&original.id).unwrap().unwrap();assert_eq!(unchanged.contributions,original.contributions);
        let encrypted=session.encrypted_database_bytes().unwrap();drop(session);store.restore_database(&subject.id,"test-password",&encrypted).unwrap();let mut restored=store.unlock(&subject.id,"test-password").unwrap();assert_eq!(all(&restored).unwrap().len(),2);assert_eq!(calculate_monthly(&mut restored,request).unwrap().result.employee_deductions,250.into());
        let mut views=vec![original.clone()];crate::work_progress::mark_recalculation(&restored,&mut views).unwrap();assert!(!views[0].needs_recalculation);
        let mut edited=all(&restored).unwrap().into_iter().find(|r|r.id=="default").unwrap();edited.base=Some(5000.into());edited.employee_percent=Some(5.into());edited.employer_percent=Some(10.into());save(&mut restored,edited).unwrap();
        crate::work_progress::mark_recalculation(&restored,&mut views).unwrap();assert!(views[0].needs_recalculation);assert_eq!(views[0].result.employee_deductions,250.into());
        let mut unaffected=new.clone();crate::work_progress::mark_recalculation(&restored,std::slice::from_mut(&mut unaffected)).unwrap();assert!(!unaffected.needs_recalculation);
        let stored:crate::app_commands::MonthlySnapshot=restored.get(Collection::Calculations,&original.id).unwrap().unwrap();assert!(!stored.needs_recalculation);assert_eq!(stored.result.employee_deductions,250.into());
        let mut updated=calculate_monthly(&mut restored,original.input.clone()).unwrap();crate::work_progress::mark_recalculation(&restored,std::slice::from_mut(&mut updated)).unwrap();assert!(!updated.needs_recalculation);
        let mut salary=plan.clone();salary.base_salary="11000".into();crate::app_commands::save_salary_plan(&mut restored,salary).unwrap();crate::work_progress::mark_recalculation(&restored,std::slice::from_mut(&mut updated)).unwrap();assert!(updated.needs_recalculation);let mut refreshed=calculate_monthly(&mut restored,original.input.clone()).unwrap();crate::work_progress::mark_recalculation(&restored,std::slice::from_mut(&mut refreshed)).unwrap();assert!(!refreshed.needs_recalculation);
        delete_configuration(&mut restored,"default").unwrap();assert!(all(&restored).unwrap().iter().all(|r|r.id!="default"));crate::work_progress::mark_recalculation(&restored,std::slice::from_mut(&mut refreshed)).unwrap();assert!(refreshed.needs_recalculation);assert_eq!(restored.get::<crate::app_commands::MonthlySnapshot>(Collection::Calculations,&original.id).unwrap().unwrap().result.employee_deductions,250.into());
        let deleted_backup=restored.encrypted_database_bytes().unwrap();drop(restored);store.restore_database(&subject.id,"test-password",&deleted_backup).unwrap();let restored=store.unlock(&subject.id,"test-password").unwrap();assert!(all(&restored).unwrap().iter().all(|r|r.id!="default"));
        drop(restored);std::fs::remove_dir_all(root).unwrap();
    }
}
#[cfg(test)]
mod import_tests {
    use super::*;
    #[test]
    fn restoration_checks_conflicts_and_withdrawal_falls_back_to_defaults(){
        let root=std::env::temp_dir().join(format!("contribution-import-{}",Uuid::new_v4()));let store=payroll_storage::Store::new(&root).unwrap();let subject=store.create_subject("导入配置","1234","上海市","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
        let employee=Employee{id:"E1".into(),name:"虚构员工".into(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"0".into(),effective_from:String::new()};session.upsert(Collection::Employees,&employee.id,&employee).unwrap();
        save(&mut session,Configuration{id:"default".into(),employee_id:None,project:"住房公积金".into(),start:"2026-01".into(),end:None,enabled:true,base:Some(10000.into()),employee_percent:Some(5.into()),employer_percent:Some(5.into())}).unwrap();
        let row=payroll_import::ValidatedRow{sheet:"CSV".into(),row:2,fields:std::collections::BTreeMap::from([("employee_id".into(),"E1".into()),("project".into(),"住房公积金".into()),("start".into(),"2026-01".into()),("employee_percent".into(),"0".into())])};
        let batch=crate::app_commands::ImportBatch{id:"batch".into(),subject_id:subject.id.clone(),filename:"test.csv".into(),sha256:"hash".into(),sheet:"CSV".into(),kind:"Social".into(),row_count:1,withdrawn_at:None};session.import_batch("batch","test.csv","hash",b"fixture",&batch,&[(2,serde_json::to_string(&row).unwrap())],&[]).unwrap();
        assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,0.into());
        crate::app_commands::set_import_status(&mut session,"batch",true).unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,500.into());
        crate::app_commands::set_import_status(&mut session,"batch",false).unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,0.into());
        crate::app_commands::set_import_status(&mut session,"batch",true).unwrap();
        save(&mut session,Configuration{id:"special".into(),employee_id:Some("E1".into()),project:"住房公积金".into(),start:"2026-01".into(),end:None,enabled:true,base:Some(12000.into()),employee_percent:None,employer_percent:None}).unwrap();
        assert!(crate::app_commands::set_import_status(&mut session,"batch",false).is_err());assert!(session.get::<crate::app_commands::ImportBatch>(Collection::ImportBatches,"batch").unwrap().unwrap().withdrawn_at.is_some());
        assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,600.into());
        delete_configuration(&mut session,"special").unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,500.into());
        crate::app_commands::set_import_status(&mut session,"batch",false).unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,0.into());delete_configuration(&mut session,"import:batch:2").unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,500.into());
        crate::app_commands::set_import_status(&mut session,"batch",true).unwrap();crate::app_commands::set_import_status(&mut session,"batch",false).unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,500.into());drop(session);std::fs::remove_dir_all(root).unwrap();
    }
}

pub(crate) fn snapshot_configurations(records:&[Configuration],employee:&str,month:&str)->Vec<Configuration>{records.iter().filter(|r|(r.employee_id.is_none()||r.employee_id.as_deref()==Some(employee))&&r.start.as_str()<=month&&r.end.as_deref().is_none_or(|end|end>=month)).cloned().collect()}

#[cfg(test)]
mod editing_tests {
 use super::*;
 use crate::app_commands::{save_salary_plan,set_import_status,SalaryPlan,ImportBatch};
 use std::collections::BTreeMap;
 #[test]
 fn imported_salary_edits_check_gaps_and_follow_batch_lifecycle(){
  let root=std::env::temp_dir().join(format!("salary-edit-{}",Uuid::new_v4()));let store=payroll_storage::Store::new(&root).unwrap();let subject=store.create_subject("编辑测试","1234","上海市","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
  let employee=Employee{id:"E1".into(),name:"虚构员工".into(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"0".into(),effective_from:String::new()};session.upsert(Collection::Employees,&employee.id,&employee).unwrap();
  let row=payroll_import::ValidatedRow{sheet:"CSV".into(),row:2,fields:BTreeMap::from([("employee_id".into(),"E1".into()),("effective_from".into(),"2026-01-01".into()),("effective_to".into(),"2026-06-30".into()),("base_salary".into(),"10000".into())])};
  let batch=ImportBatch{id:"salary-batch".into(),subject_id:subject.id.clone(),filename:"salary.csv".into(),sha256:"salary-edit".into(),sheet:"CSV".into(),kind:"SalaryPlans".into(),row_count:1,withdrawn_at:None};session.import_batch(&batch.id,&batch.filename,&batch.sha256,b"fixture",&batch,&[(2,serde_json::to_string(&row).unwrap())],&[]).unwrap();
  let mut first=crate::import_inputs::plans(&session).unwrap().remove(0);first.base_salary="12000".into();save_salary_plan(&mut session,first.clone()).unwrap();assert_eq!(crate::import_inputs::plans(&session).unwrap().len(),1);assert_eq!(crate::batch::salary_for(&session,&employee,"2026-06").unwrap(),12000.into());
  let next=SalaryPlan{id:String::new(),employee_id:"E1".into(),effective_from:"2026-07".into(),effective_to:None,base_salary:"13000".into(),note:String::new()};let mut next=save_salary_plan(&mut session,next).unwrap();
  first.effective_to=Some("2026-05".into());assert!(save_salary_plan(&mut session,first.clone()).err().unwrap().contains("缺失月份"));first.effective_to=Some("2026-07".into());assert!(save_salary_plan(&mut session,first).err().unwrap().contains("重叠"));next.effective_from="2026-08".into();assert!(save_salary_plan(&mut session,next).is_err());
  set_import_status(&mut session,&batch.id,true).unwrap();assert!(crate::batch::salary_for(&session,&employee,"2026-06").is_err());set_import_status(&mut session,&batch.id,false).unwrap();assert_eq!(crate::batch::salary_for(&session,&employee,"2026-06").unwrap(),12000.into());
  drop(session);std::fs::remove_dir_all(root).unwrap();
 }
 #[test]
 fn employee_edits_never_mutate_default_or_other_employees(){
  let root=std::env::temp_dir().join(format!("employee-edit-{}",Uuid::new_v4()));let store=payroll_storage::Store::new(&root).unwrap();let subject=store.create_subject("员工隔离","1234","上海市","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
  let employee=Employee{id:"E1".into(),name:"虚构员工".into(),department:String::new(),cost_center:String::new(),kind:"正式员工".into(),hire_date:"2026-01-01".into(),leave_date:String::new(),base_salary:"0".into(),effective_from:String::new()};session.upsert(Collection::Employees,&employee.id,&employee).unwrap();
  let default=save(&mut session,Configuration{id:String::new(),employee_id:None,project:"住房公积金".into(),start:"2026-01".into(),end:None,enabled:true,base:Some(10000.into()),employee_percent:Some(5.into()),employer_percent:Some(5.into())}).unwrap();
  let mut special=default.clone();special.id.clear();special.employee_id=Some("E1".into());special.base=Some(15000.into());special.employee_percent=Some(2.into());special.employer_percent=None;let mut special=save(&mut session,special).unwrap();
  assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,300.into());assert_eq!(calculate(&session,"E2","2026-10").unwrap()[0].employee_amount,500.into());
  special.base=Some(12000.into());special.employee_percent=Some(7.into());save(&mut session,special.clone()).unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,840.into());assert_eq!(session.get::<Configuration>(Collection::ContributionConfigurations,&default.id).unwrap().unwrap(),default);
  special.base=None;special.employee_percent=None;save(&mut session,special).unwrap();assert_eq!(calculate(&session,"E1","2026-10").unwrap()[0].employee_amount,500.into());
  let mut invalid=default.clone();invalid.employee_id=Some("E1".into());assert!(save(&mut session,invalid).is_err());drop(session);std::fs::remove_dir_all(root).unwrap();
 }
}

pub(crate) fn delete_configuration(session:&mut Session,id:&str)->Result<(),String>{
 let record=all(session)?.into_iter().find(|r|r.id==id).ok_or("配置已删除或失效，请刷新列表")?;
 session.upsert(Collection::ContributionDeletions,id,&serde_json::json!({"id":id,"configuration":record}))
}
#[tauri::command]
pub fn contribution_configuration_delete(state:State<AppState>,id:String)->Result<(),String>{state.with_session_mut(|session|delete_configuration(session,&id))}
