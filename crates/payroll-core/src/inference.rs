use crate::{formula::evaluate, Decimal};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample { pub employee_id:String, pub row_number:usize, pub fields:BTreeMap<String,Decimal>, pub observed:Decimal }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Difference { pub employee_id:String, pub row_number:usize, pub observed:Decimal, pub calculated:Option<Decimal> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Confidence { Matched, Ambiguous, Insufficient }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate { pub expression:String, pub matched:usize, pub total:usize, pub differences:Vec<Difference>, pub confidence:Confidence, pub evidence:String }

/// Compare restricted candidate expressions against independent historical observations.
/// A numerical match is evidence, never automatic approval of a payroll rule.
pub fn infer_numeric(samples:&[Sample], expressions:&[String], tolerance:Decimal)->Vec<Candidate>{
    let mut candidates=Vec::new();
    for expression in expressions{
        let mut matched=0;let mut differences=Vec::new();let mut evaluable=0;
        for sample in samples{
            match evaluate(expression,&sample.fields){
                Ok(value)=>{evaluable+=1;let rounded=value.round_dp(2);if (rounded-sample.observed).abs()<=tolerance{matched+=1;}else{differences.push(Difference{employee_id:sample.employee_id.clone(),row_number:sample.row_number,observed:sample.observed,calculated:Some(rounded)});}},
                Err(_)=>differences.push(Difference{employee_id:sample.employee_id.clone(),row_number:sample.row_number,observed:sample.observed,calculated:None}),
            }
        }
        let confidence=if evaluable!=samples.len()||samples.is_empty(){Confidence::Insufficient}else{Confidence::Ambiguous};
        candidates.push(Candidate{expression:expression.clone(),matched,total:samples.len(),differences,confidence,evidence:format!("{matched}/{} 行在允许差额内；未自动确认",samples.len())});
    }
    let max=candidates.iter().map(|c|c.matched).max().unwrap_or(0);
    let best_count=candidates.iter().filter(|c|c.matched==max&&c.total>0&&c.differences.iter().all(|d|d.calculated.is_some())).count();
    for candidate in &mut candidates{
        if candidate.total>0&&candidate.matched==candidate.total&&candidate.matched==max&&best_count==1 {candidate.confidence=Confidence::Matched;}
        else if candidate.differences.iter().any(|d|d.calculated.is_none()) {candidate.confidence=Confidence::Insufficient;}
    }
    candidates
}

/// Translate A1-style references for the same data row into restricted field names.
/// Functions, sheet references, ranges and cross-row references are rejected.
pub fn translate_excel_formula(formula:&str,headers:&[String],row_number:usize,field_codes:&BTreeMap<String,String>)->Result<String,String>{
    let source=formula.trim().strip_prefix('=').unwrap_or(formula.trim());
    let bytes=source.as_bytes();let mut output=String::new();let mut i=0;
    while i<bytes.len(){
        let c=bytes[i] as char;
        if c=='$'||c.is_ascii_alphabetic(){
            let begin=i;if c=='$'{i+=1;}
            let letters=i;while i<bytes.len()&&(bytes[i] as char).is_ascii_alphabetic(){i+=1;}
            if letters==i{return Err("公式引用格式不受支持".into());}
            if i<bytes.len()&&bytes[i]==b'$'{i+=1;}
            let digits=i;while i<bytes.len()&&(bytes[i] as char).is_ascii_digit(){i+=1;}
            if digits==i{return Err("公式含函数或命名引用，须人工配置".into());}
            let referenced_row=source[digits..i].parse::<usize>().map_err(|_|"公式行号无效")?;
            if referenced_row!=row_number{return Err("公式跨行引用，须人工确认".into());}
            let mut column=0usize;
            for ch in source[letters..digits].trim_end_matches('$').bytes(){column=column.checked_mul(26).and_then(|v|v.checked_add((ch.to_ascii_uppercase()-b'A'+1) as usize)).ok_or("公式列号无效")?;}
            let header=headers.get(column.saturating_sub(1)).ok_or("公式引用超出表头范围")?;
            let field=field_codes.get(header).ok_or_else(||format!("列 {header} 未映射为计算字段"))?;
            output.push_str(field);
            if begin==i{return Err("公式解析未前进".into());}
        }else if c.is_ascii_digit()||matches!(c,'+'|'-'|'*'|'/'|'('|')'|'.'|' '){output.push(c);i+=1;}
        else{return Err(format!("公式含不支持的运算符：{c}"));}
    }
    Ok(output)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn formula_translation_and_backcalc(){
        let headers=vec!["工号".into(),"姓名".into(),"固定月薪".into(),"标准日".into(),"实出勤".into(),"绩效".into()];
        let codes=BTreeMap::from([("固定月薪".into(),"base_salary".into()),("标准日".into(),"standard_days".into()),("实出勤".into(),"actual_days".into()),("绩效".into(),"performance".into())]);
        let formula=translate_excel_formula("=$C2/D2*E2+F2",&headers,2,&codes).unwrap();
        assert_eq!(formula,"base_salary/standard_days*actual_days+performance");
        assert!(translate_excel_formula("=SUM(C2:D2)",&headers,2,&codes).is_err());
        let sample=Sample{employee_id:"E1".into(),row_number:2,fields:BTreeMap::from([("base_salary".into(),Decimal::from(10000)),("standard_days".into(),Decimal::from(20)),("actual_days".into(),Decimal::from(20)),("performance".into(),Decimal::from(1000))]),observed:Decimal::from(11000)};
        assert_eq!(infer_numeric(&[sample],&[formula],Decimal::ZERO)[0].confidence,Confidence::Matched);
    }
    #[test]fn flags_ambiguity_and_missing_fields(){
        let sample=Sample{employee_id:"E1".into(),row_number:2,fields:BTreeMap::from([("base".into(),Decimal::from(100))]),observed:Decimal::from(100)};
        let result=infer_numeric(&[sample.clone()],&["base".into(),"base+0".into()],Decimal::ZERO);
        assert!(result.iter().all(|c|c.confidence==Confidence::Ambiguous));
        assert_eq!(infer_numeric(&[sample],&["missing+1".into()],Decimal::ZERO)[0].confidence,Confidence::Insufficient);
    }
}
