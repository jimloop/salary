import type { Page } from '../types';
export async function loadAllRecords<T>(fetchPage:(page:number,size:number)=>Promise<Page<T>>):Promise<T[]>{
  const records:T[]=[];
  for(let page=1;;page++){
    const result=await fetchPage(page,100);records.push(...result.records);
    if(records.length>=result.total||result.records.length===0)return records;
  }
}
export function pageRecords<T>(records:T[],page:number,size:number):Page<T>{
  const current=Math.min(page,Math.max(1,Math.ceil(records.length/size)));
  return {records:records.slice((current-1)*size,current*size),total:records.length,page:current,page_size:size};
}
