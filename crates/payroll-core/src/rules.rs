use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use crate::formula::{evaluate, references};
use crate::Decimal;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleStatus { Draft, PendingValidation, Active }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub code: String,
    pub label: String,
    pub expression: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub region: Option<String>,
    pub source: String,
    pub version: u32,
    pub status: RuleStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleTrace {
    pub code: String,
    pub expression: String,
    pub inputs: BTreeMap<String, Decimal>,
    pub result: Decimal,
}

pub fn validate(rules: &[Rule], available_fields: &BTreeSet<String>) -> Result<Vec<usize>, String> {
    let mut codes = BTreeMap::new();
    for (index, rule) in rules.iter().enumerate() {
        if rule.code.trim().is_empty() || rule.label.trim().is_empty() || rule.source.trim().is_empty() {
            return Err(format!("第 {} 条规则缺少编码、名称或来源", index + 1));
        }
        if codes.insert(rule.code.clone(), index).is_some() { return Err(format!("工资项目编码重复：{}", rule.code)); }
        if let Some(end) = &rule.valid_to { if end < &rule.valid_from { return Err(format!("{} 的结束日期早于开始日期", rule.code)); } }
    }
    let mut order = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    fn visit(index: usize, rules: &[Rule], codes: &BTreeMap<String, usize>, fields: &BTreeSet<String>, visiting: &mut BTreeSet<usize>, visited: &mut BTreeSet<usize>, order: &mut Vec<usize>) -> Result<(), String> {
        if visited.contains(&index) { return Ok(()); }
        if !visiting.insert(index) { return Err(format!("规则存在循环依赖：{}", rules[index].code)); }
        for name in references(&rules[index].expression)? {
            if let Some(dependency) = codes.get(&name) {
                visit(*dependency, rules, codes, fields, visiting, visited, order)?;
            } else if !fields.contains(&name) {
                return Err(format!("规则 {} 引用未知字段 {}", rules[index].code, name));
            }
        }
        visiting.remove(&index);
        visited.insert(index);
        order.push(index);
        Ok(())
    }
    for index in 0..rules.len() { visit(index, rules, &codes, available_fields, &mut visiting, &mut visited, &mut order)?; }
    Ok(order)
}

pub fn execute(rules: &[Rule], fields: &BTreeMap<String, Decimal>, date: &str, region: &str) -> Result<(BTreeMap<String, Decimal>, Vec<RuleTrace>), String> { execute_internal(rules,fields,date,region,false) }
pub fn execute_payroll(rules:&[Rule],fields:&BTreeMap<String,Decimal>,date:&str,region:&str)->Result<(BTreeMap<String,Decimal>,Vec<RuleTrace>),String>{execute_internal(rules,fields,date,region,true)}
fn execute_internal(rules:&[Rule],fields:&BTreeMap<String,Decimal>,date:&str,region:&str,round_to_cent:bool)->Result<(BTreeMap<String,Decimal>,Vec<RuleTrace>),String>{
    let order = validate(rules, &fields.keys().cloned().collect())?;
    let mut values = fields.clone();
    let mut traces = Vec::new();
    for index in order {
        let rule = &rules[index];
        if date < rule.valid_from.as_str() || rule.valid_to.as_deref().is_some_and(|end| date > end) || rule.region.as_deref().is_some_and(|r| r != region) { continue; }
        if rule.status != RuleStatus::Active { return Err(format!("规则 {} 未经人工确认，不能用于核算", rule.code)); }
        let inputs = references(&rule.expression)?.into_iter().map(|key| {
            values.get(&key).copied().map(|value| (key.clone(), value)).ok_or_else(|| format!("规则 {} 缺少输入 {}", rule.code, key))
        }).collect::<Result<BTreeMap<_, _>, _>>()?;
        let result = evaluate(&rule.expression, &values)?;
        let result=if round_to_cent{result.round_dp_with_strategy(2,rust_decimal::RoundingStrategy::MidpointAwayFromZero)}else{result};
        values.insert(rule.code.clone(), result);
        traces.push(RuleTrace { code: rule.code.clone(), expression: rule.expression.clone(), inputs, result });
    }
    Ok((values, traces))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule(code: &str, expression: &str) -> Rule { Rule { code: code.into(), label: code.into(), expression: expression.into(), valid_from: "2026-01-01".into(), valid_to: None, region: None, source: "内部制度".into(), version: 1, status: RuleStatus::Active } }
    #[test] fn sorts_dependencies_and_rejects_cycles() {
        let fields = BTreeSet::from(["base".into()]);
        assert_eq!(validate(&[rule("total", "part + 1"), rule("part", "base / 2")], &fields).unwrap(), vec![1, 0]);
        assert!(validate(&[rule("a", "b + 1"), rule("b", "a + 1")], &fields).unwrap_err().contains("循环"));
    }
    #[test] fn blocks_pending_rule_and_unknown_field() {
        let mut pending = rule("gross", "base + 1"); pending.status = RuleStatus::PendingValidation;
        assert!(execute(&[pending], &BTreeMap::from([("base".into(), Decimal::ONE)]), "2026-11-01", "上海").is_err());
        assert!(validate(&[rule("gross", "missing + 1")], &BTreeSet::new()).is_err());
    }
}
