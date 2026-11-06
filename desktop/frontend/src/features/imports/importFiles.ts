import { open } from '@tauri-apps/plugin-dialog';
/** Native file dialog boundary; replaced only by the isolated UI review entry. */
export const importFiles={
 choose:()=>open({multiple:false,filters:[{name:'工资数据',extensions:['csv','xlsx','xls','xlsb','ods']}]})
};
