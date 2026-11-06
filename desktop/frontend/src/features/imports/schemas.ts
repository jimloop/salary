import type { ImportKind } from '../../types';
export const schemas:Record<ImportKind,{title:string;fields:[string,string,string][]}>={
  salary_plans:{title:'薪酬方案表',fields:[['employee_id','工号','须存在于当前主体'],['effective_from','生效日期','YYYY-MM-DD，方案期间不得重叠'],['effective_to','结束日期','选填，YYYY-MM-DD，留空为长期'],['base_salary','固定月薪','人民币非负金额'],['note','备注','选填，记录制度来源和调整说明']]},
  bonus:{title:'年终奖资格与绩效表',fields:[['employee_id','工号','须存在于当前主体'],['bonus_year','奖金年度','四位年份，如 2026'],['target','基础奖金','非负金额；无资格时显式填 0'],['performance_factor','绩效系数','非负数字'],['eligible_days','符合资格天数','非负整数，不超过考核周期'],['plan_days','考核周期天数','正整数'],['minimum','保底金额','选填；填写非负金额'],['maximum','封顶金额','选填；不得低于保底']]},
  employees:{title:'员工基础表',fields:[['employee_id','工号','当前主体内唯一'],['name','姓名','员工显示名称'],['base_salary','固定月薪','人民币金额，保留两位'],['department','部门','选填'],['cost_center','成本中心','选填'],['hire_date','入职日期','选填，格式 YYYY-MM-DD'],['effective_from','薪酬生效日期','选填，格式 YYYY-MM-DD']]},
  attendance:{title:'考勤数据',fields:[['employee_id','工号','匹配已有员工'],['period','所属期','格式 YYYY-MM'],['standard_days','标准工作日','正数'],['actual_days','实出勤日','不大于标准工作日'],['additional_deduction','专项附加扣除','选填；缺失时本期个税不可准确测算']]},
  performance:{title:'绩效数据',fields:[['employee_id','工号','匹配已有员工'],['period','所属期','格式 YYYY-MM'],['performance','绩效奖金','人民币金额']]},
  adjustments:{title:'补扣款与变动',fields:[['employee_id','工号','匹配已有员工'],['period','所属期','格式 YYYY-MM'],['adjustment','调整项','正数补发，负数扣款']]},
  tax_history:{title:'历史实际个税',fields:[['employee_id','工号','匹配已有员工'],['payout_month','计划发放月','格式 YYYY-MM'],['cumulative_income','此前累计收入','当前发放月之前的累计收入'],['cumulative_exempt_income','此前累计免税收入','缺失时不可准确测算'],['cumulative_basic_deduction','此前累计基本减除费用','缺失时不可准确测算'],['cumulative_special_deduction','此前累计专项扣除','缺失时不可准确测算'],['cumulative_additional_deduction','此前累计专项附加扣除','缺失时不可准确测算'],['cumulative_other_deduction','此前累计其他扣除','缺失时不可准确测算'],['cumulative_tax_relief','此前累计减免税额','缺失时不可准确测算'],['cumulative_withheld_tax','此前实际已预扣个税','不能用计算估值替代']]},
  social:{title:'社保公积金',fields:[['employee_id','工号','匹配已有员工'],['period','所属期','格式 YYYY-MM'],['employee_contribution','员工承担','税前专项扣除'],['employer_contribution','单位承担','计入单位成本']]},
  historical_payroll:{title:'历史工资表',fields:[['employee_id','工号','匹配已有员工'],['period','所属期','格式 YYYY-MM'],['gross','应发工资','历史实际应发'],['net','实发工资','历史实际实发']]},
};

export const kindNames:Record<ImportKind,string>={employees:'员工基础表',salary_plans:'薪酬方案表',attendance:'考勤数据表',performance:'绩效数据表',social:'社保公积金表',tax_history:'历史实际发放与个税表',adjustments:'补发补扣表',bonus:'年终奖资格与绩效表',historical_payroll:'历史工资表'};
export const serverKinds:Record<ImportKind,string>={employees:'Employees',salary_plans:'SalaryPlans',attendance:'Attendance',performance:'Performance',social:'Social',tax_history:'TaxHistory',adjustments:'Adjustments',bonus:'Bonus',historical_payroll:'HistoricalPayroll'};
export const importOrder:ImportKind[]=['employees','salary_plans','attendance','performance','social','tax_history','adjustments','bonus'];
export const fieldLabel=(field:string)=>Object.values(schemas).flatMap(schema=>schema.fields).find(([key])=>key===field)?.[1]??field;
