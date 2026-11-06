use crate::app_commands::AppState;
use aes_gcm::{aead::{Aead,KeyInit},Aes256Gcm,Nonce};
use base64::{engine::general_purpose::STANDARD,Engine};
use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng,RngCore};
use serde::{Deserialize,Serialize};
use sha2::Sha256;
use std::fs;
use tauri::State;

const ITERATIONS:u32=600_000;
#[derive(Serialize,Deserialize)]
struct Envelope { schema:u32, salt:String, nonce:String, ciphertext:String }
#[derive(Serialize,Deserialize)]
struct Payload { subject_id:String, subject_name:String, database:String }

fn derive(password:&str,salt:&[u8])->[u8;32]{let mut key=[0;32];pbkdf2_hmac::<Sha256>(password.as_bytes(),salt,ITERATIONS,&mut key);key}

#[tauri::command]
pub fn backup_export(state:State<AppState>,path:String,password:String)->Result<(),String>{
    let mut guard=state.current.lock().map_err(|_|"工作空间状态异常")?;
    let session=guard.as_mut().ok_or("请先解锁主体")?;
    let verification=state.store.unlock(&session.subject.id,&password)?;drop(verification);
    let payload=Payload{subject_id:session.subject.id.clone(),subject_name:session.subject.name.clone(),database:STANDARD.encode(session.encrypted_database_bytes()?)};
    let mut salt=[0u8;16];let mut nonce=[0u8;12];OsRng.fill_bytes(&mut salt);OsRng.fill_bytes(&mut nonce);
    let key=derive(&password,&salt);
    let cipher=Aes256Gcm::new_from_slice(&key).map_err(|_|"密钥初始化失败")?;
    let plaintext=serde_json::to_vec(&payload).map_err(|e|e.to_string())?;
    let ciphertext=cipher.encrypt(Nonce::from_slice(&nonce),plaintext.as_ref()).map_err(|_|"备份加密失败")?;
    let envelope=Envelope{schema:1,salt:STANDARD.encode(salt),nonce:STANDARD.encode(nonce),ciphertext:STANDARD.encode(ciphertext)};
    let temporary=format!("{path}.tmp");
    fs::write(&temporary,serde_json::to_vec(&envelope).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    fs::rename(&temporary,&path).map_err(|e|e.to_string())?;
    session.log_action("backup_export","encrypted backup")
}

#[tauri::command]
pub fn backup_restore(state:State<AppState>,path:String,subject_id:String,password:String,confirmed_name:String)->Result<(),String>{
    if state.current.lock().map_err(|_|"工作空间状态异常")?.is_some(){return Err("恢复前请锁定工作空间".into());}
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    let envelope:Envelope=serde_json::from_slice(&bytes).map_err(|_|"备份文件格式无效")?;
    if envelope.schema!=1{return Err("备份格式版本不受支持".into());}
    let salt=STANDARD.decode(envelope.salt).map_err(|_|"备份盐值无效")?;
    let nonce=STANDARD.decode(envelope.nonce).map_err(|_|"备份随机数无效")?;
    if salt.len()!=16||nonce.len()!=12{return Err("备份加密参数无效".into());}
    let key=derive(&password,&salt);
    let cipher=Aes256Gcm::new_from_slice(&key).map_err(|_|"密钥初始化失败")?;
    let ciphertext=STANDARD.decode(envelope.ciphertext).map_err(|_|"备份密文无效")?;
    let plaintext=cipher.decrypt(Nonce::from_slice(&nonce),ciphertext.as_ref()).map_err(|_|"备份密码错误或文件损坏")?;
    let payload:Payload=serde_json::from_slice(&plaintext).map_err(|_|"备份内容无效")?;
    if payload.subject_id!=subject_id{return Err("备份主体与目标主体不一致，不能跨主体覆盖".into());}
    if payload.subject_name!=confirmed_name{return Err("主体名称确认不一致".into());}
    let database=STANDARD.decode(payload.database).map_err(|_|"备份数据库无效")?;
    state.store.restore_database(&subject_id,&password,&database)?;
    let mut restored=state.store.unlock(&subject_id,&password)?;
    restored.log_action("backup_restore","encrypted backup")
}
