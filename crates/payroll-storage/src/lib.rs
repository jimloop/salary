//! Per-subject SQLCipher storage. The unencrypted index contains only display metadata.
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubjectInfo { pub id: String, pub name: String, pub tax_suffix: String, #[serde(default)] pub region:String, #[serde(default)] pub last_opened_at:Option<u64> }

#[derive(Clone, Copy, Debug)]
pub enum Collection { Employees, SalaryPlans, Items, Rules, ImportBatches, HistoricalPayroll, Calculations, CalculationRuns, Bonuses, BonusSchemes, Issues, Audit }
impl Collection {
    fn table(self) -> &'static str { match self { Self::Employees => "employees", Self::SalaryPlans => "salary_plans", Self::Items => "items", Self::Rules => "rules", Self::ImportBatches => "import_batches", Self::HistoricalPayroll => "historical_payroll", Self::Calculations => "calculations", Self::CalculationRuns => "calculation_runs", Self::Bonuses => "bonuses", Self::BonusSchemes=>"bonus_schemes", Self::Issues => "issues", Self::Audit => "audit" } }
}

#[derive(Debug, Serialize)]
pub struct Page<T> { pub records: Vec<T>, pub total: u64, pub page: u32, pub page_size: u32 }

pub struct Store { root: PathBuf }
pub struct Session { pub subject: SubjectInfo, connection: Connection }

fn audit(tx: &Transaction<'_>, action: &str, target: &str) -> Result<(), String> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let id = Uuid::new_v4().to_string();
    let entry = serde_json::json!({"id":id,"timestamp":timestamp,"action":action,"target":target});
    tx.execute("INSERT INTO audit (id,json) VALUES (?1,?2)", params![id, entry.to_string()]).map_err(|e| e.to_string())?;
    Ok(())
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into(); fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }
    fn index_path(&self) -> PathBuf { self.root.join("subjects.json") }
    fn database_path(&self, id: &str) -> PathBuf { self.root.join(id).join("workspace.db") }
    pub fn list_subjects(&self) -> Result<Vec<SubjectInfo>, String> {
        let path = self.index_path();
        if !path.exists() { return Ok(Vec::new()); }
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
    fn save_index(&self, subjects: &[SubjectInfo]) -> Result<(), String> {
        let temporary = self.root.join("subjects.json.tmp");
        fs::write(&temporary, serde_json::to_vec(subjects).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        fs::rename(temporary, self.index_path()).map_err(|e| e.to_string())
    }
    pub fn create_subject(&self, name: &str, tax_suffix: &str, region:&str, password: &str) -> Result<SubjectInfo, String> {
        if name.trim().is_empty() || password.chars().count() < 8 || !tax_suffix.chars().all(|c| c.is_ascii_digit()) || tax_suffix.len() != 4 { return Err("主体名称、税号末四位或密码无效（密码至少 8 位）".into()); }
        let subject = SubjectInfo { id: Uuid::new_v4().to_string(), name: name.trim().into(), tax_suffix: tax_suffix.into(), region:region.trim().into(), last_opened_at:None };
        let directory = self.root.join(&subject.id);
        fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let result = (|| {
            let session = self.open_connection(subject.clone(), password)?;
            session.initialize()?;
            let mut subjects = self.list_subjects()?;
            subjects.push(subject.clone());
            self.save_index(&subjects)
        })();
        if result.is_err() { let _ = fs::remove_dir_all(&directory); }
        result.map(|_| subject)
    }
    fn open_connection(&self, subject: SubjectInfo, password: &str) -> Result<Session, String> {
        let connection = Connection::open(self.database_path(&subject.id)).map_err(|e| e.to_string())?;
        connection.pragma_update(None, "key", password).map_err(|e| e.to_string())?;
        connection.pragma_update(None, "foreign_keys", "ON").map_err(|e| e.to_string())?;
        connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())).map_err(|_| "密码错误或主体数据库损坏".to_string())?;
        connection.pragma_update(None, "journal_mode", "WAL").map_err(|e| e.to_string())?;
        connection.pragma_update(None, "secure_delete", "ON").map_err(|e| e.to_string())?;
        Ok(Session { subject, connection })
    }
    pub fn unlock(&self, id: &str, password: &str) -> Result<Session, String> {
        let subject = self.list_subjects()?.into_iter().find(|s| s.id == id).ok_or("主体不存在")?;
        let session = self.open_connection(subject, password)?;
        session.connection.query_row("SELECT value FROM app_meta WHERE key='schema_version'", [], |r| r.get::<_, String>(0)).map_err(|_| "主体数据库缺少版本信息".to_string())?;
        session.connection.execute_batch("CREATE TABLE IF NOT EXISTS bonus_schemes (id TEXT PRIMARY KEY,json TEXT NOT NULL);CREATE TABLE IF NOT EXISTS calculation_runs (id TEXT PRIMARY KEY,json TEXT NOT NULL);").map_err(|e|e.to_string())?;
        Ok(session)
    }
    pub fn mark_opened(&self,id:&str)->Result<(),String>{
        let mut subjects=self.list_subjects()?;
        let subject=subjects.iter_mut().find(|subject|subject.id==id).ok_or("主体不存在")?;
        subject.last_opened_at=Some(SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs());
        self.save_index(&subjects)
    }
    pub fn delete_subject(&self, id: &str, password: &str, confirmed_name: &str) -> Result<(), String> {
        let session = self.unlock(id, password)?;
        if session.subject.name != confirmed_name { return Err("主体名称确认不一致".into()); }
        drop(session);
        let mut subjects = self.list_subjects()?;
        subjects.retain(|s| s.id != id);
        let path = self.root.join(id);
        if !path.starts_with(&self.root) || !path.is_dir() { return Err("主体目录无效".into()); }
        fs::remove_dir_all(path).map_err(|e| e.to_string())?;
        self.save_index(&subjects)
    }
    /// Call only while the application has no open session for this subject.
    pub fn restore_database(&self,id:&str,password:&str,bytes:&[u8])->Result<(),String>{
        let current=self.unlock(id,password)?;
        current.encrypted_database_bytes()?;
        drop(current);
        let directory=self.root.join(id);
        let temporary=directory.join("restore.tmp");
        fs::write(&temporary,bytes).map_err(|e|e.to_string())?;
        let verify=(||{
            let connection=Connection::open(&temporary).map_err(|e|e.to_string())?;
            connection.pragma_update(None,"key",password).map_err(|e|e.to_string())?;
            let version:String=connection.query_row("SELECT value FROM app_meta WHERE key='schema_version'",[],|r|r.get(0)).map_err(|_|"备份密码错误或文件损坏".to_string())?;
            if version!="1"{return Err("备份数据库版本不受支持".into());}
            Ok::<(),String>(())
        })();
        if let Err(e)=verify{let _=fs::remove_file(temporary);return Err(e);}
        let database=self.database_path(id);
        let _=fs::remove_file(directory.join("workspace.db-wal"));
        let _=fs::remove_file(directory.join("workspace.db-shm"));
        let previous=directory.join("workspace.previous");
        fs::rename(&database,&previous).map_err(|e|e.to_string())?;
        if let Err(e)=fs::rename(&temporary,&database){let _=fs::rename(&previous,&database);return Err(e.to_string());}
        if let Err(e)=self.unlock(id,password){let _=fs::remove_file(&database);let _=fs::rename(&previous,&database);return Err(format!("恢复验证失败：{e}"));}
        let _=fs::remove_file(previous);
        Ok(())
    }
}

impl Session {
    fn initialize(&self) -> Result<(), String> {
        self.connection.execute_batch("BEGIN IMMEDIATE;
            CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO app_meta VALUES ('schema_version', '1');
            CREATE TABLE employees (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE salary_plans (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE items (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE rules (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE import_batches (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE historical_payroll (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE calculations (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE calculation_runs (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE bonuses (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE bonus_schemes (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE issues (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE audit (id TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE source_files (id TEXT PRIMARY KEY, filename TEXT NOT NULL, sha256 TEXT NOT NULL UNIQUE, bytes BLOB NOT NULL);
            CREATE TABLE import_rows (id TEXT PRIMARY KEY, batch_id TEXT NOT NULL, row_no INTEGER NOT NULL, json TEXT NOT NULL,
                FOREIGN KEY(batch_id) REFERENCES import_batches(id));
            COMMIT;").map_err(|e| e.to_string())
    }
    pub fn upsert<T: Serialize>(&mut self, collection: Collection, id: &str, value: &T) -> Result<(), String> {
        if matches!(collection, Collection::Audit) { return Err("审计日志只能由系统写入".into()); }
        if id.trim().is_empty() { return Err("记录标识不能为空".into()); }
        let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        tx.execute(&format!("INSERT INTO {} (id,json) VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET json=excluded.json", collection.table()), params![id,json]).map_err(|e| e.to_string())?;
        audit(&tx, "upsert", &format!("{}:{id}", collection.table()))?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn insert_immutable<T: Serialize>(&mut self, collection: Collection, id: &str, value: &T) -> Result<(), String> {
        if matches!(collection, Collection::Audit) { return Err("审计日志只能由系统写入".into()); }
        if id.trim().is_empty() { return Err("记录标识不能为空".into()); }
        let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        tx.execute(&format!("INSERT INTO {} (id,json) VALUES (?1,?2)", collection.table()), params![id,json]).map_err(|e| e.to_string())?;
        audit(&tx, "insert", &format!("{}:{id}", collection.table()))?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn get<T: for<'de> Deserialize<'de>>(&self, collection: Collection, id: &str) -> Result<Option<T>, String> {
        let json: Option<String> = self.connection.query_row(&format!("SELECT json FROM {} WHERE id=?1", collection.table()), [id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        json.map(|v| serde_json::from_str(&v).map_err(|e| e.to_string())).transpose()
    }
    pub fn page<T: for<'de> Deserialize<'de>>(&self, collection: Collection, page: u32, page_size: u32) -> Result<Page<T>, String> {
        if page == 0 || ![20,50,100].contains(&page_size) { return Err("分页参数无效".into()); }
        let table = collection.table();
        let total: i64 = self.connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).map_err(|e| e.to_string())?;
        let order = match collection { Collection::Calculations|Collection::CalculationRuns|Collection::Bonuses|Collection::ImportBatches|Collection::Audit => "rowid DESC", _ => "id" };
        let mut statement = self.connection.prepare(&format!("SELECT json FROM {table} ORDER BY {order} LIMIT ?1 OFFSET ?2")).map_err(|e| e.to_string())?;
        let offset = i64::from(page - 1) * i64::from(page_size);
        let rows = statement.query_map(params![page_size,offset], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        for row in rows { records.push(serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?); }
        Ok(Page { records, total: total as u64, page, page_size })
    }
    pub fn database_file(&self) -> Result<&Path, String> { self.connection.path().map(Path::new).ok_or("数据库路径不可用".into()) }
    pub fn encrypted_database_bytes(&self)->Result<Vec<u8>,String>{
        self.connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e|e.to_string())?;
        fs::read(self.database_file()?).map_err(|e|e.to_string())
    }
    pub fn imported_data(&self,include_withdrawn:bool)->Result<Vec<serde_json::Value>,String>{
        let mut statement=self.connection.prepare("SELECT r.batch_id,r.json FROM import_rows r JOIN import_batches b ON b.id=r.batch_id WHERE (?1=1 OR json_extract(b.json,'$.withdrawn_at') IS NULL) ORDER BY b.rowid DESC,r.row_no").map_err(|e|e.to_string())?;
        let rows=statement.query_map([include_withdrawn],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).map_err(|e|e.to_string())?;let mut result=Vec::new();for row in rows{let (id,data)=row.map_err(|e|e.to_string())?;let data:serde_json::Value=serde_json::from_str(&data).map_err(|e|e.to_string())?;result.push(serde_json::json!({"batch_id":id,"data":data}));}Ok(result)
    }
    pub fn imported_rows(&self,kind:&str)->Result<Vec<serde_json::Value>,String>{
        let mut statement=self.connection.prepare("SELECT r.json FROM import_rows r JOIN import_batches b ON r.batch_id=b.id WHERE json_extract(b.json,'$.kind')=?1 AND json_extract(b.json,'$.withdrawn_at') IS NULL ORDER BY b.rowid,r.row_no").map_err(|e|e.to_string())?;
        let rows=statement.query_map([kind],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
        let mut result=Vec::new();
        for row in rows{result.push(serde_json::from_str(&row.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?);}
        Ok(result)
    }
    pub fn next_calculation_version(&self,period:&str,payout_month:&str)->Result<u32,String>{
        let current:i64=self.connection.query_row("SELECT COALESCE(MAX(CAST(json_extract(json,'$.version') AS INTEGER)),0) FROM calculation_runs WHERE json_extract(json,'$.period')=?1 AND json_extract(json,'$.payout_month')=?2",params![period,payout_month],|row|row.get(0)).map_err(|e|e.to_string())?;
        u32::try_from(current+1).map_err(|_|"试算版本数量超出限制".into())
    }
    pub fn insert_calculation_run<T:Serialize>(&mut self,run_id:&str,run:&T,records:&[(String,String)],action:&str,target:&str)->Result<(),String>{
        if records.is_empty(){return Err("本次试算没有员工结果".into());}
        let run_json=serde_json::to_string(run).map_err(|e|e.to_string())?;
        let tx=self.connection.transaction().map_err(|e|e.to_string())?;
        let mut statement=tx.prepare("INSERT INTO calculations (id,json) VALUES (?1,?2)").map_err(|e|e.to_string())?;
        for (id,json) in records{statement.execute(params![id,json]).map_err(|e|e.to_string())?;}
        drop(statement);
        tx.execute("INSERT INTO calculation_runs (id,json) VALUES (?1,?2)",params![run_id,run_json]).map_err(|e|e.to_string())?;
        audit(&tx,action,target)?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn insert_rules_batch(&mut self,records:&[(String,String)])->Result<(),String>{
        let tx=self.connection.transaction().map_err(|e|e.to_string())?;
        let mut statement=tx.prepare("INSERT INTO rules (id,json) VALUES (?1,?2)").map_err(|e|e.to_string())?;
        for (id,json) in records{statement.execute(params![id,json]).map_err(|e|e.to_string())?;}
        drop(statement);audit(&tx,"import_rule_pack",&format!("{} rules",records.len()))?;tx.commit().map_err(|e|e.to_string())
    }
    pub fn import_batch(&mut self, batch_id:&str, filename:&str, sha256:&str, source:&[u8], metadata:&impl Serialize, rows:&[(usize,String)], employees:&[(String,String)]) -> Result<(),String> {
        let tx=self.connection.transaction().map_err(|e|e.to_string())?;
        let duplicate:Option<String>=tx.query_row("SELECT id FROM source_files WHERE sha256=?1",[sha256],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        if duplicate.is_some(){return Err("当前主体已经导入过相同文件；不会重复累加".into());}
        tx.execute("INSERT INTO source_files (id,filename,sha256,bytes) VALUES (?1,?2,?3,?4)",params![batch_id,filename,sha256,source]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO import_batches (id,json) VALUES (?1,?2)",params![batch_id,serde_json::to_string(metadata).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        let mut stmt=tx.prepare("INSERT INTO import_rows (id,batch_id,row_no,json) VALUES (?1,?2,?3,?4)").map_err(|e|e.to_string())?;
        for (row_no,json) in rows{stmt.execute(params![format!("{batch_id}:{row_no}"),batch_id,*row_no as i64,json]).map_err(|e|e.to_string())?;}
        drop(stmt);
        for (id,json) in employees{
            let exists:Option<String>=tx.query_row("SELECT id FROM employees WHERE id=?1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            if exists.is_some(){return Err(format!("工号 {id} 已存在；整批未写入"));}
            tx.execute("INSERT INTO employees (id,json) VALUES (?1,?2)",params![id,json]).map_err(|e|e.to_string())?;
        }
        audit(&tx,"import_file",&format!("{filename}:{batch_id}"))?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn set_import_withdrawn(&mut self,batch_id:&str,withdrawn:bool)->Result<(),String>{
        let tx=self.connection.transaction().map_err(|e|e.to_string())?;
        let json:String=tx.query_row("SELECT json FROM import_batches WHERE id=?1",[batch_id],|r|r.get(0)).map_err(|_|"导入批次不存在".to_string())?;
        let mut batch:serde_json::Value=serde_json::from_str(&json).map_err(|e|e.to_string())?;
        let was_withdrawn=batch.get("withdrawn_at").is_some_and(|value|!value.is_null());
        if was_withdrawn==withdrawn{return Err(if withdrawn{"批次已撤销"}else{"批次仍处于生效状态"}.into());}
        if batch.get("kind").and_then(serde_json::Value::as_str)==Some("Employees"){
            let mut statement=tx.prepare("SELECT json FROM import_rows WHERE batch_id=?1 ORDER BY row_no").map_err(|e|e.to_string())?;
            let rows=statement.query_map([batch_id],|row|row.get::<_,String>(0)).map_err(|e|e.to_string())?;
            let mut imported=Vec::new();
            for row in rows{
                let value:serde_json::Value=serde_json::from_str(&row.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                let fields=&value["fields"];
                let get=|key:&str|fields.get(key).and_then(serde_json::Value::as_str).unwrap_or("");
                let employee=serde_json::json!({"id":get("employee_id"),"name":get("name"),"department":get("department"),"cost_center":get("cost_center"),"kind":"正式员工","hire_date":get("hire_date"),"leave_date":"","base_salary":get("base_salary"),"effective_from":get("effective_from")});
                imported.push((get("employee_id").to_string(),employee));
            }
            drop(statement);
            for (id,expected) in imported{
                if withdrawn{
                    let current:String=tx.query_row("SELECT json FROM employees WHERE id=?1",[&id],|r|r.get(0)).map_err(|_|format!("员工 {id} 已不存在，无法撤销此批次"))?;
                    let current:serde_json::Value=serde_json::from_str(&current).map_err(|e|e.to_string())?;
                    if current!=expected{return Err(format!("员工 {id} 已被编辑，请先核对后再撤销"));}
                    for (table,path) in [("salary_plans","$.employee_id"),("calculations","$.employee_id"),("bonuses","$.request.employee_id")]{
                        let count:i64=tx.query_row(&format!("SELECT count(*) FROM {table} WHERE json_extract(json,?1)=?2"),params![path,id],|r|r.get(0)).map_err(|e|e.to_string())?;
                        if count>0{return Err(format!("员工 {id} 已有薪酬方案或计算版本，不能撤销基础资料批次"));}
                    }
                    let imported_dependencies:i64=tx.query_row("SELECT count(*) FROM import_rows r JOIN import_batches b ON b.id=r.batch_id WHERE r.batch_id<>?1 AND json_extract(r.json,'$.fields.employee_id')=?2 AND json_extract(b.json,'$.withdrawn_at') IS NULL",params![batch_id,id],|row|row.get(0)).map_err(|e|e.to_string())?;
                    if imported_dependencies>0{return Err(format!("员工 {id} 仍有生效的导入资料，请先撤销关联批次"));}
                    tx.execute("DELETE FROM employees WHERE id=?1",[&id]).map_err(|e|e.to_string())?;
                }else{
                    tx.execute("INSERT INTO employees (id,json) VALUES (?1,?2)",params![id,expected.to_string()]).map_err(|_|"恢复失败：工号已被占用，未写入任何记录".to_string())?;
                }
            }
        }
        let timestamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs();
        batch["withdrawn_at"]=if withdrawn{serde_json::json!(timestamp)}else{serde_json::Value::Null};
        tx.execute("UPDATE import_batches SET json=?2 WHERE id=?1",params![batch_id,batch.to_string()]).map_err(|e|e.to_string())?;
        audit(&tx,if withdrawn{"withdraw_import"}else{"restore_import"},batch_id)?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn log_action(&mut self,action:&str,target:&str)->Result<(),String>{
        let tx=self.connection.transaction().map_err(|e|e.to_string())?;
        audit(&tx,action,target)?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn clear_audit(&mut self)->Result<u64,String>{
        let tx=self.connection.transaction().map_err(|e|e.to_string())?;
        let count=tx.execute("DELETE FROM audit",[]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subjects_are_isolated_and_password_protected() {
        let root = std::env::temp_dir().join(format!("salary-storage-test-{}", Uuid::new_v4()));
        let store = Store::new(&root).unwrap();
        let a = store.create_subject("甲主体", "1234", "上海", "password-a").unwrap();
        let b = store.create_subject("乙主体", "5678", "杭州", "password-b").unwrap();
        let mut session = store.unlock(&a.id, "password-a").unwrap();
        session.upsert(Collection::Employees, "E001", &serde_json::json!({"name":"甲员工"})).unwrap();
        assert_eq!(session.page::<serde_json::Value>(Collection::Employees, 1, 20).unwrap().total, 1);
        let encrypted=session.encrypted_database_bytes().unwrap();
        assert!(!encrypted.windows("甲员工".len()).any(|window|window=="甲员工".as_bytes()));
        assert!(!encrypted.starts_with(b"SQLite format 3"));
        assert!(store.unlock(&a.id, "wrong-password").is_err());
        assert_eq!(store.unlock(&b.id, "password-b").unwrap().page::<serde_json::Value>(Collection::Employees, 1, 20).unwrap().total, 0);
        assert!(store.delete_subject(&a.id, "wrong-password", "甲主体").is_err());
        drop(session);
        let mut changed=store.unlock(&a.id,"password-a").unwrap();
        changed.upsert(Collection::Employees,"E001",&serde_json::json!({"name":"已修改"})).unwrap();
        drop(changed);
        store.restore_database(&a.id,"password-a",&encrypted).unwrap();
        let restored=store.unlock(&a.id,"password-a").unwrap();
        let employee:serde_json::Value=restored.get(Collection::Employees,"E001").unwrap().unwrap();
        assert_eq!(employee["name"],"甲员工");
        drop(restored);
        store.delete_subject(&a.id, "password-a", "甲主体").unwrap();
        assert_eq!(store.list_subjects().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn paginates_one_thousand_employees_and_audit_is_append_only(){
        let root=std::env::temp_dir().join(format!("salary-pagination-{}",Uuid::new_v4()));
        let store=Store::new(&root).unwrap();
        let subject=store.create_subject("分页主体","4321","上海","password-c").unwrap();
        let mut session=store.unlock(&subject.id,"password-c").unwrap();
        for index in 0..1000{
            let id=format!("E{index:04}");
            session.upsert(Collection::Employees,&id,&serde_json::json!({"id":id})).unwrap();
        }
        let first:Page<serde_json::Value>=session.page(Collection::Employees,1,20).unwrap();
        let last:Page<serde_json::Value>=session.page(Collection::Employees,10,100).unwrap();
        assert_eq!(first.total,1000);
        assert_eq!(first.records.len(),20);
        assert_eq!(last.records.len(),100);
        assert_eq!(last.records.last().unwrap()["id"],"E0999");
        assert_eq!(session.page::<serde_json::Value>(Collection::Audit,1,20).unwrap().total,1000);
        assert!(session.upsert(Collection::Audit,"fake",&serde_json::json!({})).is_err());
        drop(session);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn clearing_audit_only_affects_current_subject(){
        let root=std::env::temp_dir().join(format!("salary-audit-clear-{}",Uuid::new_v4()));
        let store=Store::new(&root).unwrap();
        let first=store.create_subject("主体甲","1234","上海","password-a").unwrap();
        let second=store.create_subject("主体乙","5678","北京","password-b").unwrap();
        let mut one=store.unlock(&first.id,"password-a").unwrap();
        let mut two=store.unlock(&second.id,"password-b").unwrap();
        one.upsert(Collection::Employees,"E1",&serde_json::json!({"name":"甲"})).unwrap();
        two.log_action("test","second").unwrap();
        assert_eq!(one.clear_audit().unwrap(),1);
        assert_eq!(one.page::<serde_json::Value>(Collection::Audit,1,20).unwrap().total,0);
        assert!(one.get::<serde_json::Value>(Collection::Employees,"E1").unwrap().is_some());
        assert_eq!(two.page::<serde_json::Value>(Collection::Audit,1,20).unwrap().total,1);
        drop(one);drop(two);fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn import_batch_can_be_withdrawn_and_restored_atomically(){
        let root=std::env::temp_dir().join(format!("salary-withdraw-{}",Uuid::new_v4()));
        let store=Store::new(&root).unwrap();
        let subject=store.create_subject("撤销主体","2468","上海","password-d").unwrap();
        let mut session=store.unlock(&subject.id,"password-d").unwrap();
        let row=serde_json::json!({"sheet":"CSV","row":2,"fields":{"subject_id":subject.id,"employee_id":"E1","period":"2026-10","actual_days":"20"}}).to_string();
        let metadata=serde_json::json!({"id":"batch-1","kind":"Attendance","row_count":1});
        session.import_batch("batch-1","attendance.csv","hash-1",b"source",&metadata,&[(2,row)],&[]).unwrap();
        assert_eq!(session.imported_rows("Attendance").unwrap().len(),1);
        session.set_import_withdrawn("batch-1",true).unwrap();
        assert!(session.imported_rows("Attendance").unwrap().is_empty());
        assert!(session.set_import_withdrawn("batch-1",true).is_err());
        session.set_import_withdrawn("batch-1",false).unwrap();
        assert_eq!(session.imported_rows("Attendance").unwrap().len(),1);
        assert_eq!(session.page::<serde_json::Value>(Collection::Audit,1,20).unwrap().total,3);
        drop(session);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn employee_import_withdrawal_preserves_edited_records(){
        let root=std::env::temp_dir().join(format!("salary-employee-withdraw-{}",Uuid::new_v4()));
        let store=Store::new(&root).unwrap();
        let subject=store.create_subject("员工主体","1357","上海","password-e").unwrap();
        let mut session=store.unlock(&subject.id,"password-e").unwrap();
        let row=serde_json::json!({"sheet":"CSV","row":2,"fields":{"subject_id":subject.id,"employee_id":"E1","name":"甲","base_salary":"10000"}}).to_string();
        let employee=serde_json::json!({"id":"E1","name":"甲","department":"","cost_center":"","kind":"正式员工","hire_date":"","leave_date":"","base_salary":"10000","effective_from":""});
        let metadata=serde_json::json!({"id":"batch-employee","kind":"Employees","row_count":1});
        session.import_batch("batch-employee","employees.csv","hash-employee",b"source",&metadata,&[(2,row)],&[("E1".into(),employee.to_string())]).unwrap();
        session.upsert(Collection::Employees,"E1",&serde_json::json!({"id":"E1","name":"已编辑"})).unwrap();
        assert!(session.set_import_withdrawn("batch-employee",true).is_err());
        assert!(session.get::<serde_json::Value>(Collection::Employees,"E1").unwrap().is_some());
        session.upsert(Collection::Employees,"E1",&employee).unwrap();
        let dependent=serde_json::json!({"sheet":"CSV","row":2,"fields":{"employee_id":"E1","effective_from":"2026-01-01","base_salary":"12000"}}).to_string();
        let plan_batch=serde_json::json!({"id":"batch-plan","kind":"SalaryPlans","row_count":1});
        session.import_batch("batch-plan","plans.csv","hash-plan",b"plans",&plan_batch,&[(2,dependent)],&[]).unwrap();
        assert!(session.set_import_withdrawn("batch-employee",true).is_err());
        session.set_import_withdrawn("batch-plan",true).unwrap();
        session.set_import_withdrawn("batch-employee",true).unwrap();
        assert!(session.get::<serde_json::Value>(Collection::Employees,"E1").unwrap().is_none());
        session.set_import_withdrawn("batch-employee",false).unwrap();
        assert_eq!(session.get::<serde_json::Value>(Collection::Employees,"E1").unwrap().unwrap()["name"],"甲");
        drop(session);
        fs::remove_dir_all(root).unwrap();
    }
}
