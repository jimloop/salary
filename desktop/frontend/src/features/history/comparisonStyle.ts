export const comparisonLabels={changed:'有差异',same:'一致',incomplete:'数据不完整',added:'仅新版本有',removed:'仅原版本有'} as const;

export function metricTone(metric:{delta:string|null}):'changed'|'same'|'incomplete'{
  return metric.delta===null?'incomplete':Number(metric.delta)===0?'same':'changed';
}
