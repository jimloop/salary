use crate::{app_commands::Item,payroll_rules};
use payroll_storage::{Collection,Session};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone,Serialize)]pub struct ItemView{#[serde(flatten)]pub item:Item,pub source:String,pub rule_kind:String,pub formula:String,pub editable_rule:bool}
pub fn defaults()->Vec<Item>{[
 ("fixed_salary","计薪固定薪","收入","计税","按主体制度确认基数"),
 ("performance_pay","绩效奖金","收入","计税","按主体制度确认基数"),
 ("adjustment_pay","补发补扣","税前调整","计入税前收入","按项目口径确认"),
 ("employee_deductions","员工社保公积金承担","员工扣款","专项扣除","员工承担"),
 ("employer_contributions","单位社保公积金承担","雇主成本","不从员工实发扣除","单位承担"),
 ("gross","应发合计","汇总","工资薪金收入合计","不重复计入基数"),
 ("withholding","预估个税","员工扣款","税额测算","不计入"),
 ("net","预估实发","汇总","税后实发","不计入"),
 ("employer_cost","单位总成本","汇总","不作为员工收入重复计税","不计入")
].into_iter().map(|(code,name,category,tax,social)|Item{code:code.into(),name:name.into(),category:category.into(),tax_attribute:tax.into(),social_attribute:social.into(),rounding:"项目到分".into()}).collect()}
pub fn catalog(session:&Session,period:&str,region:&str)->Result<Vec<ItemView>,String>{
 let date=format!("{period}-01");chrono::NaiveDate::parse_from_str(&date,"%Y-%m-%d").map_err(|_|"所属期格式错误")?;
 let mut entries:BTreeMap<String,(Item,String)>=defaults().into_iter().map(|item|(item.code.clone(),(item,"default".into()))).collect();let mut page=1;loop{let rows=session.page::<Item>(Collection::Items,page,100)?;for mut item in rows.records{item.category=match item.category.as_str(){"扣款"=>"员工扣款".into(),"单位成本"=>"雇主成本".into(),_=>item.category};entries.insert(item.code.clone(),(item,"custom".into()));}if page as u64*100>=rows.total{break}page+=1;}
 let region=if region.is_empty(){session.subject.region.as_str()}else{region};
 let rules=payroll_rules::selected(&payroll_rules::all(session)?,&date,region);
 // A configured rule is a real calculation project even if no separate item form was saved.
 for rule in &rules{entries.entry(rule.code.clone()).or_insert((Item{code:rule.code.clone(),name:rule.label.clone(),category:"自定义项目".into(),tax_attribute:"以应发公式及税费口径为准".into(),social_attribute:"按项目口径确认".into(),rounding:"项目到分".into()},"rule".into()));}
 let mut result:Vec<ItemView>=entries.into_values().map(|(item,source)|{let rule=rules.iter().find(|rule|rule.code==item.code);let editable_rule=!["withholding","net","employer_cost","employee_deductions","employer_contributions"].contains(&item.code.as_str());let (rule_kind,formula)=if let Some(rule)=rule{(if rule.version==0{"默认规则".into()}else{format!("自定义 v{}",rule.version)},rule.expression.clone())}else if !editable_rule{("默认规则（系统汇总）".into(),match item.code.as_str(){"withholding"=>"累计预扣法，依赖历史实际税务基线","net"=>"应发 − 员工承担 − 本期个税",_=>"应发 + 单位承担"}.into())}else{("未配置生效规则".into(),String::new())};ItemView{item,source,rule_kind,formula,editable_rule}}).collect();
 let order:Vec<String>=defaults().into_iter().map(|item|item.code).collect();result.sort_by_key(|value|order.iter().position(|code|code==&value.item.code).unwrap_or(usize::MAX));Ok(result)
}

#[cfg(test)]mod tests{
 use super::*;use payroll_core::rules::{Rule,RuleStatus};
 #[test]fn actual_default_projects_are_visible_without_saved_item_rows(){
  let root=std::env::temp_dir().join(format!("items-catalog-{}",uuid::Uuid::new_v4()));let store=payroll_storage::Store::new(root.clone()).unwrap();let subject=store.create_subject("项目主体","1234","上海","test-password").unwrap();let mut session=store.unlock(&subject.id,"test-password").unwrap();
  assert_eq!(session.page::<Item>(Collection::Items,1,20).unwrap().total,0);let initial=catalog(&session,"2026-10","上海").unwrap();assert_eq!(initial.len(),9);assert_eq!(initial[0].item.code,"fixed_salary");assert!(initial.iter().all(|item|item.source=="default"));
  let mut edited=defaults().remove(0);edited.name="主体固定薪".into();session.upsert(Collection::Items,"fixed_salary",&edited).unwrap();let rule=Rule{code:"fixed_salary".into(),label:"主体固定薪".into(),expression:"base_salary".into(),valid_from:"2026-10-01".into(),valid_to:Some("2026-10-31".into()),region:Some("上海".into()),source:"主体制度".into(),version:1,status:RuleStatus::Active};session.insert_immutable(Collection::Rules,"fixed_salary-1",&rule).unwrap();
  let current=catalog(&session,"2026-10","上海").unwrap();assert_eq!(current.len(),9);let item=current.iter().find(|item|item.item.code=="fixed_salary").unwrap();assert_eq!(item.item.name,"主体固定薪");assert_eq!(item.source,"custom");assert_eq!(item.rule_kind,"自定义 v1");assert_eq!(catalog(&session,"2026-11","上海").unwrap().iter().find(|item|item.item.code=="fixed_salary").unwrap().rule_kind,"默认规则");drop(session);let _=std::fs::remove_dir_all(root);
 }
}
