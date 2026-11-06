const importFields = [
  {
    "id": "employees",
    "name": "员工基础表",
    "stage": "基础数据",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体标识一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "当前主体内唯一",
        "required": true
      },
      {
        "key": "name",
        "label": "姓名",
        "description": "员工显示名称",
        "required": true
      },
      {
        "key": "base_salary",
        "label": "固定月薪",
        "description": "人民币金额，保留两位",
        "required": true
      },
      {
        "key": "department",
        "label": "部门",
        "description": "选填",
        "required": false
      },
      {
        "key": "cost_center",
        "label": "成本中心",
        "description": "选填",
        "required": false
      },
      {
        "key": "hire_date",
        "label": "入职日期",
        "description": "选填，格式 YYYY-MM-DD",
        "required": false
      },
      {
        "key": "effective_from",
        "label": "薪酬生效日期",
        "description": "选填，格式 YYYY-MM-DD",
        "required": false
      }
    ]
  },
  {
    "id": "salary_plans",
    "name": "薪酬方案表",
    "stage": "基础数据",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "须存在于当前主体",
        "required": true
      },
      {
        "key": "effective_from",
        "label": "生效日期",
        "description": "YYYY-MM-DD，方案期间不得重叠",
        "required": true
      },
      {
        "key": "effective_to",
        "label": "结束日期",
        "description": "选填，YYYY-MM-DD，留空为长期",
        "required": false
      },
      {
        "key": "base_salary",
        "label": "固定月薪",
        "description": "人民币非负金额",
        "required": true
      },
      {
        "key": "note",
        "label": "备注",
        "description": "选填，记录制度来源和调整说明",
        "required": false
      }
    ]
  },
  {
    "id": "attendance",
    "name": "考勤数据表",
    "stage": "月薪输入",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体标识一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "匹配已有员工",
        "required": true
      },
      {
        "key": "period",
        "label": "所属期",
        "description": "格式 YYYY-MM",
        "required": true
      },
      {
        "key": "standard_days",
        "label": "标准工作日",
        "description": "正数",
        "required": true
      },
      {
        "key": "actual_days",
        "label": "实出勤日",
        "description": "不大于标准工作日",
        "required": true
      },
      {
        "key": "additional_deduction",
        "label": "专项附加扣除",
        "description": "选填；缺失时本期个税不可准确测算",
        "required": false
      }
    ]
  },
  {
    "id": "performance",
    "name": "绩效数据表",
    "stage": "月薪输入",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体标识一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "匹配已有员工",
        "required": true
      },
      {
        "key": "period",
        "label": "所属期",
        "description": "格式 YYYY-MM",
        "required": true
      },
      {
        "key": "performance",
        "label": "绩效奖金",
        "description": "人民币金额",
        "required": true
      }
    ]
  },
  {
    "id": "social",
    "name": "社保公积金表",
    "stage": "税费输入",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体标识一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "匹配已有员工",
        "required": true
      },
      {
        "key": "period",
        "label": "所属期",
        "description": "格式 YYYY-MM",
        "required": true
      },
      {
        "key": "employee_contribution",
        "label": "员工承担",
        "description": "税前专项扣除",
        "required": true
      },
      {
        "key": "employer_contribution",
        "label": "单位承担",
        "description": "计入单位成本",
        "required": true
      }
    ]
  },
  {
    "id": "tax_history",
    "name": "历史实际发放与个税表",
    "stage": "税费输入",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体标识一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "匹配已有员工",
        "required": true
      },
      {
        "key": "payout_month",
        "label": "计划发放月",
        "description": "格式 YYYY-MM",
        "required": true
      },
      {
        "key": "cumulative_income",
        "label": "此前累计收入",
        "description": "当前发放月之前的累计收入",
        "required": true
      },
      {
        "key": "cumulative_exempt_income",
        "label": "此前累计免税收入",
        "description": "缺失时不可准确测算",
        "required": false
      },
      {
        "key": "cumulative_basic_deduction",
        "label": "此前累计基本减除费用",
        "description": "缺失时不可准确测算",
        "required": false
      },
      {
        "key": "cumulative_special_deduction",
        "label": "此前累计专项扣除",
        "description": "缺失时不可准确测算",
        "required": false
      },
      {
        "key": "cumulative_additional_deduction",
        "label": "此前累计专项附加扣除",
        "description": "缺失时不可准确测算",
        "required": false
      },
      {
        "key": "cumulative_other_deduction",
        "label": "此前累计其他扣除",
        "description": "缺失时不可准确测算",
        "required": false
      },
      {
        "key": "cumulative_tax_relief",
        "label": "此前累计减免税额",
        "description": "缺失时不可准确测算",
        "required": false
      },
      {
        "key": "cumulative_withheld_tax",
        "label": "此前实际已预扣个税",
        "description": "不能用计算估值替代",
        "required": true
      }
    ]
  },
  {
    "id": "adjustments",
    "name": "补发补扣表",
    "stage": "月薪输入",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体标识一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "匹配已有员工",
        "required": true
      },
      {
        "key": "period",
        "label": "所属期",
        "description": "格式 YYYY-MM",
        "required": true
      },
      {
        "key": "adjustment",
        "label": "调整项",
        "description": "正数补发，负数扣款",
        "required": true
      }
    ]
  },
  {
    "id": "bonus",
    "name": "年终奖资格与绩效表",
    "stage": "年终奖输入",
    "fields": [
      {
        "key": "subject_id",
        "label": "主体ID",
        "description": "必须与当前主体一致",
        "required": true
      },
      {
        "key": "employee_id",
        "label": "工号",
        "description": "须存在于当前主体",
        "required": true
      },
      {
        "key": "bonus_year",
        "label": "奖金年度",
        "description": "四位年份，如 2026",
        "required": true
      },
      {
        "key": "target",
        "label": "基础奖金",
        "description": "非负金额；无资格时显式填 0",
        "required": true
      },
      {
        "key": "performance_factor",
        "label": "绩效系数",
        "description": "非负数字",
        "required": true
      },
      {
        "key": "eligible_days",
        "label": "符合资格天数",
        "description": "非负整数，不超过考核周期",
        "required": true
      },
      {
        "key": "plan_days",
        "label": "考核周期天数",
        "description": "正整数",
        "required": true
      },
      {
        "key": "minimum",
        "label": "保底金额",
        "description": "选填；填写非负金额",
        "required": false
      },
      {
        "key": "maximum",
        "label": "封顶金额",
        "description": "选填；不得低于保底",
        "required": false
      }
    ]
  }
];
