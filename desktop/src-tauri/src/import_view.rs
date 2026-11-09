use crate::app_commands::AppState;
use payroll_storage::{Collection,Page,Session};
use payroll_import::{ImportKind,ValidatedRow};
use payroll_core::Decimal;
use serde::{Deserialize,Serialize};
use std::collections::{BTreeMap,BTreeSet};
use tauri::State;
#[derive(Clone,Deserialize)]pub struct DataFilter{pub period:String,pub payout_month:String,pub year:String,#[serde(default)]pub employee_id:String,#[serde(default)]pub batch_id:String}
#[derive(Clone,Serialize)]pub struct DataRow{pub batch_id:String,pub filename:String,pub kind:String,pub sheet:String,pub row:usize,pub fields:BTreeMap<String,String>}
#[derive(Serialize)]pub struct FieldTotal{pub value:Decimal,pub missing_count:usize}
#[derive(Serialize)]pub struct UploadStatus{pub kind:String,pub row_count:usize,pub batch_count:usize,pub employee_count:usize,pub duplicate_count:usize,pub totals:BTreeMap<String,FieldTotal>}
fn valid_filter(filter:&DataFilter)->Result<(),String>{for value in [&filter.period,&filter.payout_month]{if chrono::NaiveDate::parse_from_str(&format!("{value}-01"),"%Y-%m-%d").is_err(){return Err("所属期及计划发放月须为 YYYY-MM".into());}}if filter.year.len()!=4||filter.year.parse::<u32>().is_err(){return Err("奖金年度须为四位年份".into());}Ok(())}
fn matches(row:&DataRow,filter:&DataFilter)->bool{
 if !filter.batch_id.is_empty(){return row.batch_id==filter.batch_id;}
 if !filter.employee_id.is_empty()&&row.fields.get("employee_id")!=Some(&filter.employee_id){return false;}
 let get=|key:&str|row.fields.get(key).map(String::as_str).unwrap_or("");
 match row.kind.as_str(){
  "Employees"=>true,
  "SalaryPlans"=>get("effective_from").get(..7).unwrap_or("")<=filter.period.as_str()&&(get("effective_to").is_empty()||get("effective_to").get(..7).unwrap_or("")>=filter.period.as_str()),
  "Social" if get("project").is_empty()=>false,
  "Social"=>get("start")<=filter.period.as_str()&&(get("end").is_empty()||get("end")>=filter.period.as_str()),
  "TaxHistory"=>get("payout_month")==filter.payout_month,
  "Bonus"=>get("bonus_year")==filter.year,
  _=>get("period")==filter.period
 }
}
pub fn data_rows(session:&Session,filter:&DataFilter)->Result<Vec<DataRow>,String>{
 valid_filter(filter)?;let mut batches=Vec::new();let mut page=1;loop{let result=session.page::<serde_json::Value>(Collection::ImportBatches,page,100)?;batches.extend(result.records);if page as u64*100>=result.total{break}page+=1;}
 let batches:BTreeMap<String,serde_json::Value>=batches.into_iter().filter_map(|batch|batch["id"].as_str().map(str::to_owned).map(|id|(id,batch))).collect();
 // Storage joins the source batch in the same encrypted subject database.
 let mut result=Vec::new();for value in session.imported_data(!filter.batch_id.is_empty())?{let batch_id=value["batch_id"].as_str().ok_or("导入行缺少批次")?.to_owned();let batch=batches.get(&batch_id);if let Some(batch)=batch{let row:ValidatedRow=serde_json::from_value(value["data"].clone()).map_err(|e|e.to_string())?;let item=DataRow{batch_id,filename:batch["filename"].as_str().unwrap_or("").into(),kind:batch["kind"].as_str().unwrap_or("").into(),sheet:row.sheet,row:row.row,fields:row.fields};if matches(&item,filter){result.push(item)}}}Ok(result)
}
fn total_fields(kind:&str)->&'static[&'static str]{match kind{"Employees"|"SalaryPlans"=>&["base_salary"],"Attendance"=>&["standard_days","actual_days","additional_deduction"],"Performance"=>&["performance"],"Adjustments"=>&["adjustment"],"Social"=>&["base"],"TaxHistory"=>&["cumulative_income","cumulative_special_deduction","cumulative_withheld_tax"],"Bonus"=>&["target"],_=>&["gross","net"]}}
pub fn overview(session:&Session,filter:&DataFilter)->Result<Vec<UploadStatus>,String>{
 let rows=data_rows(session,filter)?;let kinds=["Employees","SalaryPlans","Attendance","Performance","Social","TaxHistory","Adjustments","Bonus","HistoricalPayroll"];
 kinds.into_iter().map(|kind|{
  let selected:Vec<&DataRow>=rows.iter().filter(|row|row.kind==kind).collect();
  let employees:BTreeSet<&str>=selected.iter().filter_map(|row|row.fields.get("employee_id").map(String::as_str)).collect();
  let batches:BTreeSet<&str>=selected.iter().map(|row|row.batch_id.as_str()).collect();
  let totals=total_fields(kind).iter().map(|field|{
   let mut value=Decimal::ZERO;let mut missing_count=0;
   for row in &selected{match row.fields.get(*field).and_then(|text|text.replace(',',"").parse::<Decimal>().ok()){Some(number)=>value=value.checked_add(number).ok_or_else(||format!("字段 {field} 汇总超出金额范围"))?,None=>missing_count+=1}}
   Ok(((*field).to_string(),FieldTotal{value,missing_count}))
  }).collect::<Result<BTreeMap<_,_>,String>>()?;
  Ok(UploadStatus{kind:kind.into(),row_count:selected.len(),batch_count:batches.len(),employee_count:employees.len(),duplicate_count:if ["Employees","SalaryPlans","Social"].contains(&kind){0}else{selected.len().saturating_sub(employees.len())},totals})
 }).collect()
}
#[derive(Serialize)]pub struct EmployeeImportData{pub employee:Option<crate::app_commands::Employee>,pub groups:BTreeMap<String,Vec<DataRow>>}
pub fn employee_data(session:&Session,filter:&DataFilter)->Result<EmployeeImportData,String>{if filter.employee_id.is_empty(){return Err("请选择员工".into());}let employee=session.get(Collection::Employees,&filter.employee_id)?;let mut groups=BTreeMap::<String,Vec<DataRow>>::new();for row in data_rows(session,filter)?{groups.entry(row.kind.clone()).or_default().push(row);}Ok(EmployeeImportData{employee,groups})}
#[tauri::command]pub fn imports_employee_data(state:State<AppState>,filter:DataFilter)->Result<EmployeeImportData,String>{state.with_session(|session|employee_data(session,&filter))}
#[tauri::command]pub fn imports_overview(state:State<AppState>,filter:DataFilter)->Result<Vec<UploadStatus>,String>{state.with_session(|session|overview(session,&filter))}
#[tauri::command]pub fn imports_data_page(state:State<AppState>,kind:ImportKind,filter:DataFilter,page:u32,page_size:u32)->Result<Page<DataRow>,String>{state.with_session(|session|{
 if page==0||![20,50,100].contains(&page_size){return Err("分页参数无效".into());}let kind=format!("{kind:?}");let rows:Vec<DataRow>=data_rows(session,&filter)?.into_iter().filter(|row|row.kind==kind).collect();let total=rows.len() as u64;let current=page.min(((total+u64::from(page_size)-1)/u64::from(page_size)).max(1) as u32);Ok(Page{records:rows.into_iter().skip(((current-1)*page_size) as usize).take(page_size as usize).collect(),total,page:current,page_size})
})}

#[cfg(test)]mod tests{
 use super::*;use payroll_storage::Store;
 fn insert(session:&mut Session,id:&str,kind:&str,fields:Vec<BTreeMap<String,String>>){let data:Vec<_>=fields.into_iter().enumerate().map(|(index,fields)|(index+2,serde_json::to_string(&ValidatedRow{sheet:"CSV".into(),row:index+2,fields}).unwrap())).collect();let metadata=serde_json::json!({"id":id,"kind":kind,"filename":format!("{id}.csv"),"row_count":data.len(),"withdrawn_at":null});session.import_batch(id,&format!("{id}.csv"),id,b"fixture",&metadata,&data,&[]).unwrap();}
 fn row(employee:&str,period:&str,actual:&str)->BTreeMap<String,String>{BTreeMap::from([("employee_id".into(),employee.into()),("period".into(),period.into()),("standard_days".into(),"22".into()),("actual_days".into(),actual.into())])}
 fn filter()->DataFilter{DataFilter{period:"2026-10".into(),payout_month:"2026-11".into(),year:"2026".into(),employee_id:String::new(),batch_id:String::new()}}
 #[test]fn summaries_match_period_person_withdrawal_and_source(){
  let root=std::env::temp_dir().join(format!("import-view-{}",uuid::Uuid::new_v4()));let store=Store::new(root.clone()).unwrap();let a=store.create_subject("甲","1111","上海","password-a").unwrap();let b=store.create_subject("乙","2222","杭州","password-b").unwrap();let mut session=store.unlock(&a.id,"password-a").unwrap();
  insert(&mut session,"attendance","Attendance",vec![row("E1","2026-10","21"),row("E2","2026-10","22"),row("E1","2026-11","20")]);
  let current=filter();let values=overview(&session,&current).unwrap();let att=values.iter().find(|value|value.kind=="Attendance").unwrap();assert_eq!((att.row_count,att.employee_count,att.batch_count),(2,2,1));assert_eq!(att.totals["actual_days"].value,Decimal::from(43));assert_eq!(att.totals["additional_deduction"].missing_count,2);
  let mut person=current.clone();person.employee_id="E1".into();let rows=data_rows(&session,&person).unwrap();let personal=employee_data(&session,&person).unwrap();assert_eq!(personal.groups["Attendance"].len(),1);assert_eq!(personal.groups["Attendance"][0].fields["actual_days"],"21");assert!(!personal.groups.contains_key("Performance"));assert_eq!(rows.len(),1);assert_eq!(rows[0].row,2);assert_eq!(rows[0].filename,"attendance.csv");
  let mut next=current.clone();next.period="2026-11".into();assert_eq!(data_rows(&session,&next).unwrap().len(),1);
  insert(&mut session,"duplicate","Attendance",vec![row("E1","2026-10","1")]);assert_eq!(overview(&session,&current).unwrap().iter().find(|value|value.kind=="Attendance").unwrap().duplicate_count,1);
  session.set_import_withdrawn("attendance",true).unwrap();session.set_import_withdrawn("duplicate",true).unwrap();assert_eq!(overview(&session,&current).unwrap().iter().find(|value|value.kind=="Attendance").unwrap().row_count,0);
  let mut original=current.clone();original.batch_id="attendance".into();assert_eq!(data_rows(&session,&original).unwrap().len(),3);
  session.set_import_withdrawn("attendance",false).unwrap();assert_eq!(data_rows(&session,&current).unwrap().len(),2);let other=store.unlock(&b.id,"password-b").unwrap();assert!(data_rows(&other,&current).unwrap().is_empty());drop(other);drop(session);let _=std::fs::remove_dir_all(root);
 }
 #[test]fn payout_year_and_effective_dates_are_not_mixed(){
  let root=std::env::temp_dir().join(format!("import-dates-{}",uuid::Uuid::new_v4()));let store=Store::new(root.clone()).unwrap();let subject=store.create_subject("日期主体","1111","上海","password-date").unwrap();let mut session=store.unlock(&subject.id,"password-date").unwrap();
  insert(&mut session,"tax","TaxHistory",vec![BTreeMap::from([("employee_id".into(),"E1".into()),("payout_month".into(),"2026-11".into()),("cumulative_income".into(),"90000".into())]),BTreeMap::from([("employee_id".into(),"E1".into()),("payout_month".into(),"2026-12".into()),("cumulative_income".into(),"100000".into())])]);
  insert(&mut session,"bonus","Bonus",vec![BTreeMap::from([("employee_id".into(),"E1".into()),("bonus_year".into(),"2025".into())]),BTreeMap::from([("employee_id".into(),"E1".into()),("bonus_year".into(),"2026".into())])]);
  insert(&mut session,"plans","SalaryPlans",vec![BTreeMap::from([("employee_id".into(),"E1".into()),("effective_from".into(),"2026-01-01".into()),("effective_to".into(),"2026-09-30".into())]),BTreeMap::from([("employee_id".into(),"E1".into()),("effective_from".into(),"2026-10-15".into()),("effective_to".into(),"2026-12-31".into())])]);
  let rows=data_rows(&session,&filter()).unwrap();assert_eq!(rows.iter().filter(|row|row.kind=="TaxHistory").count(),1);assert_eq!(rows.iter().find(|row|row.kind=="TaxHistory").unwrap().fields["cumulative_income"],"90000");assert_eq!(rows.iter().filter(|row|row.kind=="Bonus").count(),1);assert_eq!(rows.iter().filter(|row|row.kind=="SalaryPlans").count(),1);drop(session);let _=std::fs::remove_dir_all(root);
 }
}
