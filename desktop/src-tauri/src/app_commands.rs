use payroll_core::rules::Rule;
use payroll_storage::{Collection, Page, Session, Store, SubjectInfo};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;
use payroll_core::{monthly::{calculate_pay, CurrentTaxInput, PayResult, TaxBaseline}, Decimal};
use payroll_core::bonus::{award, compare_tax, separate_bonus_tax, BonusAmount, BonusPlan, BonusTaxComparison};
use std::str::FromStr;
use uuid::Uuid;
use payroll_import::{read_file, validate, ImportKind, RowError, Validation};
use payroll_core::inference::{infer_numeric, translate_excel_formula, Candidate, Sample};
use std::{collections::{BTreeMap,BTreeSet}, fs, path::Path};

pub struct AppState { pub store: Store, pub current: Mutex<Option<Session>> }
impl AppState {
    pub(crate) fn with_session<T>(&self, f: impl FnOnce(&Session) -> Result<T,String>) -> Result<T,String> {
        let guard = self.current.lock().map_err(|_| "工作空间状态异常")?;
        f(guard.as_ref().ok_or("请先解锁主体")?)
    }
    pub(crate) fn with_session_mut<T>(&self, f: impl FnOnce(&mut Session) -> Result<T,String>) -> Result<T,String> {
        let mut guard = self.current.lock().map_err(|_| "工作空间状态异常")?;
        f(guard.as_mut().ok_or("请先解锁主体")?)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: String, pub name: String, pub department: String, pub cost_center: String,
    pub kind: String, pub hire_date: String, pub leave_date: String,
    pub base_salary: String, pub effective_from: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Item { pub code: String, pub name: String, pub category: String, pub tax_attribute: String, pub social_attribute: String, pub rounding: String }
#[derive(Clone, Serialize, Deserialize)]
pub struct SalaryPlan { pub id:String,pub employee_id:String,pub effective_from:String,pub effective_to:Option<String>,pub base_salary:String,pub note:String }

#[derive(Clone, Serialize, Deserialize)]
pub struct MonthlyRequest {
    #[serde(default)] pub region:String,
    pub employee_id: String, pub period: String, pub payout_month: String,
    pub standard_days: String, pub actual_days: String, pub performance: String, pub adjustment: String,
    pub employee_deductions: String, pub employer_contributions: String,
    pub tax_baseline: Option<TaxBaseline>, pub basic_deduction: Option<String>,
    pub additional_deduction: Option<String>, pub other_deduction: Option<String>,
    pub exempt_income:Option<String>, pub tax_relief:Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct MonthlySnapshot {
    #[serde(default)] pub calculation_origin:String,
    #[serde(default)] pub applied_rules:Vec<Rule>,
    pub id: String, pub subject_id: String, pub employee_id: String, pub period: String, pub payout_month: String,
    pub fixed_salary: Decimal, pub performance: Decimal, pub adjustment: Decimal,
    pub result: PayResult, pub input: MonthlyRequest,
    #[serde(default)] pub rule_trace:Vec<payroll_core::rules::RuleTrace>,
}
#[derive(Clone, Serialize)]
pub struct SheetPreview { pub name:String, pub headers:Vec<String>, pub sample:Vec<Vec<String>>, pub row_count:usize, pub formula_count:usize }
#[derive(Clone, Serialize)]
pub struct ImportPreview { pub filename:String, pub sha256:String, pub sheets:Vec<SheetPreview> }
#[derive(Clone, Serialize)]
pub struct ImportCheck { pub total_rows:usize, pub error_count:usize, pub errors:Vec<RowError> }
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportBatch { pub id:String, pub subject_id:String, pub filename:String, pub sha256:String, pub sheet:String, pub kind:String, pub row_count:usize, #[serde(default)] pub withdrawn_at:Option<u64> }
#[derive(Clone, Serialize)]
pub struct InferenceReport { pub filename:String, pub sha256:String, pub analyzed_rows:usize, pub skipped_rows:usize, pub candidates:Vec<Candidate> }
#[derive(Clone, Serialize, Deserialize)]
pub struct BonusRequest { #[serde(default)] pub scheme_id:Option<String>, pub employee_id:String, pub payout_month:String, pub target:String, pub performance_factor:String, pub eligible_days:u32, pub plan_days:u32, pub minimum:Option<String>, pub maximum:Option<String>, pub tax_baseline:Option<TaxBaseline>, pub selected_method:Option<String> }
#[derive(Clone, Serialize, Deserialize)]
pub struct BonusSnapshot { #[serde(default)] pub scheme:Option<crate::bonus_scheme::BonusScheme>, pub id:String, pub subject_id:String, pub request:BonusRequest, pub amount:BonusAmount, pub tax_comparison:Option<BonusTaxComparison>, pub selected_tax:Option<Decimal>, pub net:Option<Decimal>, pub policy_source:String }
#[derive(Clone, Serialize)]
pub struct VersionDiff { pub left_id:String, pub right_id:String, pub employee_id:String, pub gross_delta:Decimal, pub net_delta:Option<Decimal>, pub changed_inputs:Vec<String> }
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditEntry { pub id:String, pub timestamp:u64, pub action:String, pub target:String }
pub(crate) fn amount(value: &str, field: &str) -> Result<Decimal,String> { Decimal::from_str(value).map_err(|_|format!("{field} 不是有效金额")) }

fn validate_employee_refs(session:&Session,kind:ImportKind,validation:&mut Validation)->Result<(),String>{
    if kind==ImportKind::Employees{return Ok(());}
    let unique:BTreeSet<String>=validation.rows.iter().filter_map(|row|row.fields.get("employee_id").cloned()).collect();
    let mut missing=BTreeSet::new();
    for id in unique{if !id.is_empty()&&session.get::<Employee>(Collection::Employees,&id)?.is_none(){missing.insert(id);}}
    for row in &validation.rows{
        if row.fields.get("employee_id").is_some_and(|id|missing.contains(id)){
            validation.errors.push(RowError{sheet:row.sheet.clone(),row:row.row,field:"employee_id".into(),reason:"工号在当前主体不存在，请先导入员工基础表".into()});
        }
    }
    crate::import_inputs::validate_plans(session,kind,validation)?;
    Ok(())
}

#[tauri::command] pub fn subjects_list(state: State<AppState>) -> Result<Vec<SubjectInfo>,String> { state.store.list_subjects() }
#[tauri::command] pub fn subjects_create(state: State<AppState>,name:String,tax_suffix:String,region:String,password:String) -> Result<SubjectInfo,String> { state.store.create_subject(&name,&tax_suffix,&region,&password) }
#[tauri::command] pub fn subjects_unlock(state: State<AppState>,id:String,password:String) -> Result<(),String> {
    let mut guard=state.current.lock().map_err(|_| "工作空间状态异常")?;
    *guard=None;
    let mut session=state.store.unlock(&id,&password)?;
    session.log_action("unlock","workspace")?;
    state.store.mark_opened(&id)?;
    *guard=Some(session);
    Ok(())
}
#[tauri::command] pub fn subjects_lock(state: State<AppState>) -> Result<(),String> { *state.current.lock().map_err(|_| "工作空间状态异常")?=None;Ok(()) }
#[tauri::command] pub fn subjects_delete(state: State<AppState>,id:String,password:String,confirmed_name:String) -> Result<(),String> {
    let guard=state.current.lock().map_err(|_| "工作空间状态异常")?;
    if guard.as_ref().is_some_and(|session| session.subject.id==id) {return Err("请先锁定此主体再删除".into());}
    state.store.delete_subject(&id,&password,&confirmed_name)
}

#[tauri::command] pub fn employees_page(state: State<AppState>,page:u32,page_size:u32,query:String,kind:Option<String>) -> Result<Page<Employee>,String> {
    if page==0||![20,50,100].contains(&page_size){return Err("分页参数无效".into());}
    state.with_session(|s| {
        let kind=kind.unwrap_or_default();
        if query.trim().is_empty()&&kind.is_empty() {return s.page(Collection::Employees,page,page_size);}
        let all=s.page::<Employee>(Collection::Employees,1,100)?;
        // Search is limited to the current subject. At the daily target (<1000 employees),
        // the JSON index can be replaced by SQL indexed columns before the 10k benchmark.
        let mut records=all.records;
        let mut next=2;
        while records.len()<(all.total as usize) { let batch=s.page::<Employee>(Collection::Employees,next,100)?; if batch.records.is_empty(){break;} records.extend(batch.records);next+=1; }
        let needle=query.trim().to_lowercase();
        let matches:Vec<Employee>=records.into_iter().filter(|e|[&e.id,&e.name,&e.department].iter().any(|v|v.to_lowercase().contains(&needle))&&(kind.is_empty()||e.kind==kind)).collect();
        let start=((page.saturating_sub(1))*page_size) as usize;
        let total=matches.len() as u64;
        Ok(Page{records:matches.into_iter().skip(start).take(page_size as usize).collect(),total,page,page_size})
    })
}
#[tauri::command] pub fn employees_save(state: State<AppState>,employee:Employee) -> Result<(),String> {
    if employee.id.trim().is_empty()||employee.name.trim().is_empty()||employee.hire_date.trim().is_empty() {return Err("工号、姓名和入职日期必填".into());}
    if employee.base_salary.parse::<payroll_core::Decimal>().map_err(|_|"固定月薪不是有效金额")?.is_sign_negative() {return Err("固定月薪不能为负".into());}
    state.with_session_mut(|s|s.upsert(Collection::Employees,&employee.id,&employee))
}
#[tauri::command] pub fn salary_plans_page(state:State<AppState>,employee_id:String,page:u32,page_size:u32)->Result<Page<SalaryPlan>,String>{
    state.with_session_mut(|session|{
        if page==0||![20,50,100].contains(&page_size){return Err("分页参数无效".into());}
        let mut records:Vec<SalaryPlan>=crate::import_inputs::plans(session)?.into_iter().filter(|plan|plan.employee_id==employee_id).collect();
        records.sort_by(|a,b|b.effective_from.cmp(&a.effective_from));
        let total=records.len() as u64;let start=((page-1)*page_size) as usize;
        Ok(Page{records:records.into_iter().skip(start).take(page_size as usize).collect(),total,page,page_size})
    })
}
#[tauri::command] pub fn salary_plans_save(state:State<AppState>,mut plan:SalaryPlan)->Result<SalaryPlan,String>{
    state.with_session_mut(|session|{
        if plan.id.starts_with("import:"){return Err("导入的薪酬方案请通过撤销原批次后重新导入修订".into());}
        if plan.employee_id.trim().is_empty()||plan.effective_from.len()!=10{return Err("工号和方案生效日期必填".into());}
        if plan.effective_to.as_deref().is_some_and(|end|end<plan.effective_from.as_str()){return Err("方案结束日期早于生效日期".into());}
        if amount(&plan.base_salary,"固定月薪")?.is_sign_negative(){return Err("固定月薪不能为负".into());}
        let _:Employee=session.get(Collection::Employees,&plan.employee_id)?.ok_or("员工不存在")?;
        for old in crate::import_inputs::plans(session)?{if old.id!=plan.id&&crate::import_inputs::overlap(&old,&plan){return Err(format!("与 {} 起的已有薪酬方案期间冲突",old.effective_from));}}
        if plan.id.is_empty(){plan.id=Uuid::new_v4().to_string();}
        session.upsert(Collection::SalaryPlans,&plan.id,&plan)?;
        Ok(plan)
    })
}
#[tauri::command] pub fn items_page(state:State<AppState>,page:u32,page_size:u32,category:Option<String>,period:Option<String>,region:Option<String>)->Result<Page<crate::item_catalog::ItemView>,String>{state.with_session(|session|{
 if page==0||![20,50,100].contains(&page_size){return Err("分页参数无效".into());}let category=category.unwrap_or_default();let period=period.unwrap_or_else(||"2026-10".into());let region=region.unwrap_or_else(||session.subject.region.clone());let rows:Vec<_>=crate::item_catalog::catalog(session,&period,&region)?.into_iter().filter(|value|category.is_empty()||value.item.category==category).collect();let total=rows.len() as u64;let current=page.min(((total+u64::from(page_size)-1)/u64::from(page_size)).max(1) as u32);Ok(Page{records:rows.into_iter().skip(((current-1)*page_size) as usize).take(page_size as usize).collect(),total,page:current,page_size})
})}
#[tauri::command] pub fn items_save(state: State<AppState>,item:Item) -> Result<(),String> {
    if item.code.trim().is_empty()||item.name.trim().is_empty(){return Err("项目编码与名称必填".into());}
    state.with_session_mut(|s|s.upsert(Collection::Items,&item.code,&item))
}
#[tauri::command] pub fn rules_page(state: State<AppState>,page:u32,page_size:u32) -> Result<Page<Rule>,String> {state.with_session(|s|s.page(Collection::Rules,page,page_size))}
#[tauri::command] pub fn rules_save(state: State<AppState>,mut rule:Rule) -> Result<(),String> {
    if rule.version==0{return Err("版本 0 为系统默认规则保留".into());}
    state.with_session_mut(|s|{
        let mut latest=BTreeMap::<String,Rule>::new();let mut page=1;let mut max_version=0;
        loop{let batch=s.page::<Rule>(Collection::Rules,page,100)?;
            for old in batch.records{if old.code==rule.code{max_version=max_version.max(old.version);}
                if latest.get(&old.code).is_none_or(|current|old.version>current.version){latest.insert(old.code.clone(),old);}}
            if page as u64*100>=batch.total{break;}page+=1;
        }
        if let Some(old)=s.get::<Rule>(Collection::Rules,&format!("{}-{}",rule.code,rule.version))?{
            if serde_json::to_value(&old).map_err(|e|e.to_string())?==serde_json::to_value(&rule).map_err(|e|e.to_string())?{return Ok(());}
            rule.version=max_version.checked_add(1).ok_or("规则版本超出范围")?;
        }
        latest.insert(rule.code.clone(),rule.clone());
        crate::payroll_rules::validate_configured(&latest.into_values().collect::<Vec<_>>())?;
        s.insert_immutable(Collection::Rules,&format!("{}-{}",rule.code,rule.version),&rule)
    })
}

#[tauri::command] pub fn payroll_calculate(state: State<AppState>,request:MonthlyRequest) -> Result<MonthlySnapshot,String> {
    state.with_session_mut(|session| {
        if request.period.len()!=7||request.payout_month.len()!=7||request.period.as_bytes().get(4)!=Some(&b'-')||request.payout_month.as_bytes().get(4)!=Some(&b'-') {return Err("所属期或发放月格式错误".into());}
        let employee:Employee=session.get(Collection::Employees,&request.employee_id)?.ok_or("当前主体没有该员工")?;
        let base=crate::batch::salary_for(session,&employee,&request.period)?;
        let standard=amount(&request.standard_days,"标准工作日")?;
        let actual=amount(&request.actual_days,"实出勤日")?;
        if standard<=Decimal::ZERO||actual<Decimal::ZERO||actual>standard {return Err("工作日数量无效".into());}
        let performance=amount(&request.performance,"绩效奖金")?;
        let adjustment=amount(&request.adjustment,"调整项")?;
        let employee_deductions=amount(&request.employee_deductions,"员工承担")?;
        let employer_contributions=amount(&request.employer_contributions,"单位承担")?;
        let region=if request.region.is_empty(){session.subject.region.clone()}else{request.region.clone()};let date=format!("{}-01",request.period);
        let rules=crate::payroll_rules::selected(&crate::payroll_rules::all(session)?,&date,&region);
        let fields=BTreeMap::from([("base_salary".into(),base),("standard_days".into(),standard),("actual_days".into(),actual),("performance".into(),performance),("adjustment".into(),adjustment),("employee_contribution".into(),employee_deductions),("employer_contribution".into(),employer_contributions)]);
        let values=crate::payroll_rules::calculate(&rules,&fields,&date,&region)?;let fixed=values.fixed;let performance=values.performance;let adjustment=values.adjustment;let employee_deductions=values.deductions;let employer_contributions=values.employer;let gross=values.gross;

        let current=if request.tax_baseline.is_some() {
            Some(CurrentTaxInput{taxable_income:gross,exempt_income:amount(request.exempt_income.as_deref().ok_or("缺少本期免税收入")?,"免税收入")?,
                basic_deduction:amount(request.basic_deduction.as_deref().ok_or("缺少本期基本减除费用")?,"基本减除费用")?,
                special_deduction:employee_deductions,
                additional_deduction:amount(request.additional_deduction.as_deref().ok_or("缺少本期专项附加扣除")?,"专项附加扣除")?,
                other_deduction:amount(request.other_deduction.as_deref().ok_or("缺少本期其他扣除")?,"其他扣除")?,tax_relief:amount(request.tax_relief.as_deref().ok_or("缺少本期减免税额")?,"减免税额")?})
        }else{None};
        let tax=request.tax_baseline.as_ref().zip(current.as_ref());
        let result=calculate_pay(gross,employee_deductions,employer_contributions,tax)?;
        let snapshot=MonthlySnapshot{calculation_origin:"manual".into(),applied_rules:rules.clone(),id:Uuid::new_v4().to_string(),subject_id:session.subject.id.clone(),employee_id:request.employee_id.clone(),period:request.period.clone(),payout_month:request.payout_month.clone(),fixed_salary:fixed,performance,adjustment,result,input:request,rule_trace:values.trace};
        let run=crate::export_runs::CalculationRun::new(session,&snapshot.period,&snapshot.payout_month,"single",vec![snapshot.id.clone()])?;
        let record=serde_json::to_string(&snapshot).map_err(|e|e.to_string())?;
        let audit_target=serde_json::json!({"run_version":run.version,"employee_count":1}).to_string();
        session.insert_calculation_run(&run.id,&run,&[(snapshot.id.clone(),record)],"insert",&audit_target)?;
        Ok(snapshot)
    })
}
#[tauri::command] pub fn payroll_versions(state: State<AppState>,page:u32,page_size:u32) -> Result<Page<MonthlySnapshot>,String> { state.with_session(|s|s.page(Collection::Calculations,page,page_size)) }

#[tauri::command] pub fn imports_preview(state: State<AppState>,path:String) -> Result<ImportPreview,String> {
    state.with_session(|_| {
        let file=read_file(Path::new(&path))?;
        Ok(ImportPreview{filename:file.filename,sha256:file.sha256,sheets:file.sheets.into_iter().map(|s|SheetPreview{name:s.name,headers:s.headers,sample:s.rows.iter().take(5).cloned().collect(),row_count:s.rows.len(),formula_count:s.formulas.len()}).collect()})
    })
}
#[tauri::command] pub fn imports_validate(state: State<AppState>,path:String,kind:ImportKind,sheet:String,mapping:BTreeMap<String,String>) -> Result<ImportCheck,String> {
    state.with_session(|session|{
        let file=read_file(Path::new(&path))?;
        let mut result=validate(&file,kind,&session.subject.id,&sheet,&mapping)?;
        validate_employee_refs(session,kind,&mut result)?;
        Ok(ImportCheck{total_rows:result.rows.len(),error_count:result.errors.len(),errors:result.errors.into_iter().take(200).collect()})
    })
}
#[tauri::command] pub fn imports_commit(state: State<AppState>,path:String,kind:ImportKind,sheet:String,mapping:BTreeMap<String,String>) -> Result<ImportBatch,String> {
    state.with_session_mut(|session|{
        let file=read_file(Path::new(&path))?;
        let mut result=validate(&file,kind,&session.subject.id,&sheet,&mapping)?;
        validate_employee_refs(session,kind,&mut result)?;
        if !result.errors.is_empty(){return Err(format!("有 {} 行级错误；请先修复，数据库未写入",result.errors.len()));}
        let id=Uuid::new_v4().to_string();
        let batch=ImportBatch{id:id.clone(),subject_id:session.subject.id.clone(),filename:file.filename.clone(),sha256:file.sha256.clone(),sheet:sheet.clone(),kind:format!("{kind:?}"),row_count:result.rows.len(),withdrawn_at:None};
        let rows=result.rows.iter().map(|r|Ok((r.row,serde_json::to_string(r).map_err(|e|e.to_string())?))).collect::<Result<Vec<_>,String>>()?;
        let source=fs::read(&path).map_err(|e|e.to_string())?;
        let employees=if kind==ImportKind::Employees {result.rows.iter().map(|row|{
            let get=|key:&str|row.fields.get(key).cloned().unwrap_or_default();
            let employee=Employee{id:get("employee_id"),name:get("name"),department:get("department"),cost_center:get("cost_center"),kind:"正式员工".into(),hire_date:get("hire_date"),leave_date:String::new(),base_salary:get("base_salary"),effective_from:get("effective_from")};
            if amount(&employee.base_salary,"固定月薪")?.is_sign_negative(){return Err("固定月薪不能为负".into());}
            Ok((employee.id.clone(),serde_json::to_string(&employee).map_err(|e|e.to_string())?))
        }).collect::<Result<Vec<_>,String>>()?}else{Vec::new()};
        session.import_batch(&id,&file.filename,&file.sha256,&source,&batch,&rows,&employees)?;
        Ok(batch)
    })
}
#[tauri::command] pub fn imports_page(state: State<AppState>,page:u32,page_size:u32) -> Result<Page<ImportBatch>,String> {state.with_session(|s|s.page(Collection::ImportBatches,page,page_size))}
#[tauri::command] pub fn imports_set_withdrawn(state:State<AppState>,batch_id:String,withdrawn:bool)->Result<(),String>{state.with_session_mut(|s|s.set_import_withdrawn(&batch_id,withdrawn))}

#[tauri::command] pub fn inference_analyze(state: State<AppState>,path:String,sheet:String,observed_header:String,mapping:BTreeMap<String,String>) -> Result<InferenceReport,String> {
    state.with_session(|_|{
        let file=read_file(Path::new(&path))?;
        let selected=file.sheets.iter().find(|s|s.name==sheet).ok_or("工作表不存在")?;
        let observed_col=selected.headers.iter().position(|h|h==&observed_header).ok_or("未选择历史实发或应发列")?;
        let codes:BTreeMap<String,String>=mapping.iter().filter(|(_,header)|!header.is_empty()).map(|(field,header)|(header.clone(),field.clone())).collect();
        let mut samples=Vec::new();let mut skipped=0;
        for (index,row) in selected.rows.iter().enumerate(){
            let observed=row.get(observed_col).and_then(|v|Decimal::from_str(v.trim()).ok());
            if let Some(observed)=observed{
                let fields:BTreeMap<String,Decimal>=mapping.iter().filter_map(|(field,header)|selected.headers.iter().position(|h|h==header).and_then(|col|row.get(col)).and_then(|v|Decimal::from_str(v.trim()).ok()).map(|value|(field.clone(),value))).collect();
                let employee_id=selected.headers.iter().position(|h|h=="工号").and_then(|col|row.get(col)).cloned().unwrap_or_else(||format!("行{}",index+2));
                samples.push(Sample{employee_id,row_number:index+2,fields,observed});
            }else{skipped+=1;}
        }
        let mut expressions=Vec::new();
        for formula in selected.formulas.iter().filter(|cell|cell.column as usize==observed_col+1){
            if let Ok(expression)=translate_excel_formula(&formula.expression,&selected.headers,formula.row as usize,&codes){if !expressions.contains(&expression){expressions.push(expression);}}
        }
        for expression in ["base_salary / standard_days * actual_days + performance + adjustment","base_salary / standard_days * actual_days + performance","base_salary + performance + adjustment","base_salary"]{if !expressions.iter().any(|v|v==expression){expressions.push(expression.into());}}
        let candidates=infer_numeric(&samples,&expressions,Decimal::new(1,2));
        Ok(InferenceReport{filename:file.filename,sha256:file.sha256,analyzed_rows:samples.len(),skipped_rows:skipped,candidates})
    })
}

#[tauri::command] pub fn bonus_calculate(state: State<AppState>,request:BonusRequest) -> Result<BonusSnapshot,String> {
    state.with_session_mut(|session|{
        let _:Employee=session.get(Collection::Employees,&request.employee_id)?.ok_or("当前主体没有该员工")?;
        if request.payout_month.as_str()>"2027-12" {return Err("单独计税政策配置仅到 2027-12，请先核实新政策".into());}
        let scheme=request.scheme_id.as_deref().map(|id|session.get::<crate::bonus_scheme::BonusScheme>(Collection::BonusSchemes,id)?.ok_or_else(||"奖金方案不存在".to_string())).transpose()?;
        let plan=BonusPlan{target:amount(&request.target,"目标奖金")?,performance_factor:amount(&request.performance_factor,"绩效系数")?,eligible_days:request.eligible_days,plan_days:request.plan_days,minimum:request.minimum.as_deref().map(|v|amount(v,"保底金额")).transpose()?,maximum:request.maximum.as_deref().map(|v|amount(v,"封顶金额")).transpose()?};
        let calculated=award(&plan)?;
        let comparison=request.tax_baseline.as_ref().map(|baseline|compare_tax(calculated.awarded,baseline)).transpose()?;
        let selected_tax=match request.selected_method.as_deref(){
            Some("separate")=>Some(separate_bonus_tax(calculated.awarded)?),
            Some("combined")=>Some(comparison.as_ref().ok_or("并入综合所得需要完整税务基线")?.combined_increment),
            None=>None,
            _=>return Err("计税方式无效".into()),
        };
        let net=selected_tax.map(|tax|calculated.awarded-tax);
        let snapshot=BonusSnapshot{scheme,id:Uuid::new_v4().to_string(),subject_id:session.subject.id.clone(),request,amount:calculated,tax_comparison:comparison,selected_tax,net,policy_source:"财政部 税务总局公告2023年第30号；单独计税配置有效至2027-12-31".into()};
        session.insert_immutable(Collection::Bonuses,&snapshot.id,&snapshot)?;
        Ok(snapshot)
    })
}
#[tauri::command] pub fn bonus_versions(state: State<AppState>,page:u32,page_size:u32) -> Result<Page<BonusSnapshot>,String> {state.with_session(|s|s.page(Collection::Bonuses,page,page_size))}
#[tauri::command] pub fn audit_page(state:State<AppState>,page:u32,page_size:u32)->Result<Page<AuditEntry>,String>{state.with_session(|s|s.page(Collection::Audit,page,page_size))}
#[tauri::command] pub fn audit_clear(state:State<AppState>)->Result<u64,String>{state.with_session_mut(|s|s.clear_audit())}

#[tauri::command] pub fn versions_compare(state: State<AppState>,left_id:String,right_id:String) -> Result<VersionDiff,String> {
    state.with_session(|session|{
        let left:MonthlySnapshot=session.get(Collection::Calculations,&left_id)?.ok_or("左侧版本不存在")?;
        let right:MonthlySnapshot=session.get(Collection::Calculations,&right_id)?.ok_or("右侧版本不存在")?;
        if left.employee_id!=right.employee_id||left.period!=right.period{return Err("仅支持同一员工、同一所属期的月薪版本对比".into());}
        let left_input=serde_json::to_value(&left.input).map_err(|e|e.to_string())?;
        let right_input=serde_json::to_value(&right.input).map_err(|e|e.to_string())?;
        let mut changed_inputs=Vec::new();
        if let (Some(a),Some(b))=(left_input.as_object(),right_input.as_object()) {for (key,value) in a{if b.get(key)!=Some(value){changed_inputs.push(key.clone());}}}
        Ok(VersionDiff{left_id,right_id,employee_id:left.employee_id,gross_delta:right.result.gross-left.result.gross,net_delta:left.result.net.zip(right.result.net).map(|(a,b)|b-a),changed_inputs})
    })
}

#[tauri::command] pub fn exports_payroll_csv(state: State<AppState>,path:String,password:String,run_id:String) -> Result<usize,String> {
    state.with_session_mut(|session|{
        let verification=state.store.unlock(&session.subject.id,&password)?;
        drop(verification);
        let (run,snapshots)=crate::export_runs::selected_snapshots(session,&run_id)?;
        let mut bytes=vec![0xef,0xbb,0xbf];
        let mut writer=csv::Writer::from_writer(Vec::new());
        let names=crate::export_runs::employee_names(session,&snapshots)?;
        writer.write_record(["工号","姓名","固定薪","绩效","调整项","应发工资","员工承担","本期预扣个税","实发工资","单位成本"]).map_err(|e|e.to_string())?;
        for snapshot in &snapshots{
            writer.write_record([snapshot.employee_id.as_str(),names.get(&snapshot.employee_id).map(String::as_str).unwrap_or(""),&snapshot.fixed_salary.to_string(),&snapshot.performance.to_string(),&snapshot.adjustment.to_string(),&snapshot.result.gross.to_string(),&snapshot.result.employee_deductions.to_string(),&snapshot.result.withholding.as_ref().map(|v|v.current_withholding.to_string()).unwrap_or_else(||"不可准确测算".into()),&snapshot.result.net.map(|v|v.to_string()).unwrap_or_else(||"不可准确测算".into()),&snapshot.result.employer_cost.to_string()]).map_err(|e|e.to_string())?;
        }
        bytes.extend(writer.into_inner().map_err(|e|e.to_string())?);
        fs::write(path,bytes).map_err(|e|e.to_string())?;
        session.log_action("export_payroll",&serde_json::json!({"run_id":run.id,"run_version":run.version,"version_ids":run.version_ids}).to_string())?;
        Ok(snapshots.len())
    })
}
