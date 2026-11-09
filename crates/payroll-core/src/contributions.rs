//! Effective-month contribution configuration and deterministic field inheritance.
use crate::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Configuration {
    pub id: String,
    pub employee_id: Option<String>,
    pub project: String,
    pub start: String,
    pub end: Option<String>,
    pub enabled: bool,
    pub base: Option<Decimal>,
    pub employee_percent: Option<Decimal>,
    pub employer_percent: Option<Decimal>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppliedField { pub value: Decimal, pub source_id: String, #[serde(default)] pub scope:String }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Contribution {
    pub project: String,
    pub base: AppliedField,
    pub employee_percent: AppliedField,
    pub employer_percent: AppliedField,
    pub employee_amount: Decimal,
    pub employer_amount: Decimal,
}
pub fn valid_month(value: &str) -> bool {
    value.len() == 7 && value.is_ascii() && value.as_bytes()[4] == b'-'
        && value[..4].bytes().all(|b| b.is_ascii_digit())
        && value[..4] != *"0000"
        && matches!(&value[5..], "01"|"02"|"03"|"04"|"05"|"06"|"07"|"08"|"09"|"10"|"11"|"12")
}
pub fn validate(records: &[Configuration]) -> Result<(), String> {
    let mut identifiers=std::collections::BTreeSet::new();
    let mut intervals=std::collections::BTreeMap::<(&str,Option<&str>),Vec<&Configuration>>::new();
    for record in records {
        if !valid_month(&record.start) || record.end.as_ref().is_some_and(|end| !valid_month(end) || end < &record.start) {
            return Err(format!("{} 有效月份无效", record.id));
        }
        if !["养老保险","医疗保险","失业保险","工伤保险","生育保险","住房公积金"].contains(&record.project.as_str()) || record.id.trim().is_empty() { return Err("请选择有效的缴费项目，配置编号不能为空".into()); }
        if record.base.is_some_and(|v| v <= Decimal::ZERO) { return Err("缴费基数必须大于零".into()); }
        for value in [record.employee_percent, record.employer_percent].into_iter().flatten() {
            if value < Decimal::ZERO || value > Decimal::from(100) { return Err("比例必须在 0%–100% 之间".into()); }
        }

        if !identifiers.insert(record.id.as_str()){return Err("配置编号重复".into());}
        intervals.entry((&record.project,record.employee_id.as_deref())).or_default().push(record);
    }
    for ((project,_),mut group) in intervals{group.sort_by_key(|record|record.start.as_str());for adjacent in group.windows(2){if adjacent[0].end.as_deref().unwrap_or("9999-12")>=adjacent[1].start.as_str(){return Err(format!("{project} 配置有效期重叠"));}}}

    Ok(())
}
pub fn resolve(records: &[Configuration], employee: &str, month: &str) -> Result<Vec<Contribution>, String> {
    if !valid_month(month) { return Err("所属月份无效".into()); }
    validate(records)?;
    let active: Vec<_> = records.iter().filter(|r| r.start.as_str() <= month && r.end.as_deref().is_none_or(|end| end >= month)).collect();
    let projects: std::collections::BTreeSet<_> = records.iter().filter(|r|r.employee_id.is_none()||r.employee_id.as_deref()==Some(employee)).map(|r| r.project.as_str()).collect();
    if projects.is_empty(){return Err(format!("员工 {employee} 在 {month} 缺少缴费配置"));}
    let mut result = Vec::new();
    for project in projects {
        let default = active.iter().copied().find(|r| r.project == project && r.employee_id.is_none());
        if default.is_some_and(|r|!r.enabled) { continue; }
        let special = active.iter().copied().find(|r| r.project == project && r.employee_id.as_deref() == Some(employee));
        let field = |get: fn(&Configuration) -> Option<Decimal>, label: &str| -> Result<AppliedField, String> {
            let source = special.filter(|r| get(r).is_some()).or(default).ok_or_else(||format!("员工 {employee} 在 {month} 缺少 {project} {label}"))?;
            Ok(AppliedField { value: get(source).ok_or_else(|| format!("员工 {employee} 在 {month} 缺少 {project} {label}"))?, source_id: source.id.clone(), scope:if source.employee_id.is_some(){"employee".into()}else{"subject".into()} })
        };
        let base = field(|r| r.base, "基数")?;
        let employee_percent = field(|r| r.employee_percent, "员工比例")?;
        let employer_percent = field(|r| r.employer_percent, "单位比例")?;
        let amount = |percent: Decimal| -> Result<Decimal,String> { Ok(base.value.checked_mul(percent).and_then(|v|v.checked_div(Decimal::from(100))).ok_or("缴费金额超出支持范围")?.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)) };
        result.push(Contribution { project: project.into(), employee_amount: amount(employee_percent.value)?, employer_amount: amount(employer_percent.value)?, base, employee_percent, employer_percent });
    }
    if records.is_empty() { return Err(format!("员工 {employee} 在 {month} 缺少主体缴费配置")); }
    Ok(result)
}
pub fn totals(rows:&[Contribution])->Result<(Decimal,Decimal),String>{
    rows.iter().try_fold((Decimal::ZERO,Decimal::ZERO),|(employee,employer),r|Ok((employee.checked_add(r.employee_amount).ok_or("员工缴费汇总超出金额范围")?,employer.checked_add(r.employer_amount).ok_or("单位缴费汇总超出金额范围")?)))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn defaults() -> Configuration { Configuration { id:"default".into(), employee_id:None, project:"住房公积金".into(), start:"2026-01".into(), end:None, enabled:true, base:Some(10000.into()), employee_percent:Some(Decimal::new(25,1)), employer_percent:Some(5.into()) } }
    #[test] fn partial_override_and_zero_preserve_sources() {
        let default = defaults();
        let mut special = default.clone(); special.id="employee".into(); special.employee_id=Some("E1".into()); special.base=None; special.employee_percent=Some(Decimal::ZERO); special.employer_percent=None;
        let applied=resolve(&[default,special],"E1","2026-10").unwrap().remove(0);
        assert_eq!(applied.employee_amount,Decimal::ZERO); assert_eq!(applied.employer_amount,500.into()); assert_eq!(applied.base.source_id,"default"); assert_eq!(applied.employee_percent.source_id,"employee");
    }
    #[test] fn defaults_and_month_boundaries() {
        let mut old=defaults(); old.end=Some("2026-06".into());
        let mut next=defaults(); next.id="next".into(); next.start="2026-07".into(); next.base=Some(12000.into());
        let records=[old,next];
        assert_eq!(resolve(&records,"E1","2026-06").unwrap()[0].employee_amount,250.into());
        assert_eq!(resolve(&records,"E1","2026-07").unwrap()[0].employee_amount,300.into());
    }
    #[test] fn project_rounding_precedes_totals_and_disabled_projects_skip_missing_fields() {
        let mut first=defaults(); first.base=Some(Decimal::new(10005,2)); first.employee_percent=Some(1.into());
        let mut second=first.clone(); second.id="second".into(); second.project="养老保险".into();
        let applied=resolve(&[first,second],"E1","2026-10").unwrap();
        assert_eq!(applied.iter().map(|r|r.employee_amount).sum::<Decimal>(),Decimal::new(200,2));
        let mut disabled=defaults(); disabled.enabled=false; disabled.base=None; disabled.employee_percent=None;
        assert!(resolve(&[disabled],"E1","2026-10").unwrap().is_empty());
    }
    #[test] fn missing_overlap_and_invalid_inputs_block() {
        let mut missing=defaults(); missing.base=None;
        assert!(resolve(&[missing],"E1","2026-10").is_err());
        let mut duplicate=defaults(); duplicate.id="duplicate".into();
        assert!(validate(&[defaults(),duplicate]).is_err());
        assert!(!valid_month("中-11")); assert!(!valid_month("0000-01")); assert!(!valid_month("2026-13")); assert!(!valid_month("2026-1"));
    }
    #[test] fn new_employee_only_project_requires_complete_values_and_does_not_affect_others(){
        let default=defaults();let mut new=defaults();new.id="injury".into();new.employee_id=Some("E1".into());new.project="工伤保险".into();new.employee_percent=Some(Decimal::ZERO);new.employer_percent=Some(Decimal::new(2,1));
        let records=[default,new.clone()];assert_eq!(resolve(&records,"E1","2026-10").unwrap().len(),2);assert_eq!(resolve(&records,"E2","2026-10").unwrap().len(),1);new.base=None;assert!(resolve(&[new],"E1","2026-10").is_err());
    }

}
