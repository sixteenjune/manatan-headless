use crate::state::{AppState, StoredRecord};
use anyhow::Result;
use serde_json::{Value, json};
use std::io::Read;
use tracing::info;
use wordbase_api::{
    Dictionary, DictionaryId, DictionaryKind, DictionaryMeta, Record,
    dict::yomitan::{Glossary, structured},
};
use zip::ZipArchive;

pub fn import_zip(state: &AppState, data: &[u8]) -> Result<String> {
    info!(
        "📦 [Import] Starting ZIP import (size: {} bytes)...",
        data.len()
    );

    let mut zip = ZipArchive::new(std::io::Cursor::new(data))?;

    // 1. Find index.json
    // FIX: Use a loop instead of iterator to avoid "captured variable cannot escape FnMut" error
    let mut index_file_name = None;
    for i in 0..zip.len() {
        if let Ok(file) = zip.by_index(i) {
            if file.name().ends_with("index.json") {
                index_file_name = Some(file.name().to_string());
                break;
            }
        }
    }

    let index_file_name =
        index_file_name.ok_or_else(|| anyhow::anyhow!("No index.json found in zip"))?;

    let meta = {
        let mut file = zip.by_name(&index_file_name)?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        let json: Value = serde_json::from_str(&s)?;

        let name = json["title"].as_str().unwrap_or("Unknown").to_string();
        let mut dm = DictionaryMeta::new(DictionaryKind::Yomitan, name);
        dm.version = json["revision"].as_str().map(|s| s.to_string());
        dm.description = json["description"].as_str().map(|s| s.to_string());
        dm
    };

    let dict_name = meta.name.clone();

    // 2. Register Dictionary Metadata (In Memory)
    let dict_id;
    {
        let mut next_id = state.next_dict_id.write().expect("lock");
        dict_id = DictionaryId(*next_id);
        *next_id += 1;

        let mut dicts = state.dictionaries.write().expect("lock");
        dicts.insert(
            dict_id,
            Dictionary {
                id: dict_id,
                meta,
                position: 0,
            },
        );
    }

    // 3. Database Transaction Setup
    let mut conn = state.pool.get()?;
    let tx = conn.transaction()?;

    // 4. Scan for term banks and Insert
    let file_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for name in file_names {
        if name.contains("term_bank") && name.ends_with(".json") {
            info!("   -> Processing {}", name);
            let mut file = zip.by_name(&name)?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let bank: Vec<Value> = serde_json::from_str(&s).unwrap_or_default();

            let mut stmt = tx.prepare("INSERT INTO terms (term, json) VALUES (?, ?)")?;

            for entry in bank {
                if let Some(arr) = entry.as_array() {
                    let headword = arr.get(0).and_then(|v| v.as_str()).unwrap_or("");
                    let reading = arr.get(1).and_then(|v| v.as_str()).unwrap_or("");

                    let definition_arr = arr.get(5).and_then(|v| v.as_array());
                    let mut content_list = Vec::new();
                    if let Some(defs) = definition_arr {
                        for d in defs {
                            if let Some(str_def) = d.as_str() {
                                content_list.push(structured::Content::String(str_def.to_string()));
                            } else if let Some(obj_def) = d.as_object() {
                                let json_str = serde_json::to_string(&obj_def).unwrap_or_default();
                                content_list.push(structured::Content::String(json_str));
                            }
                        }
                    }

                    if headword.is_empty() {
                        continue;
                    }

                    let tags_raw = arr.get(2).and_then(|v| v.as_str()).unwrap_or("");
                    let mut tags_vec = Vec::new();
                    if !tags_raw.is_empty() {
                        for t_str in tags_raw.split_whitespace() {
                            if let Ok(tag) = serde_json::from_value(json!(t_str)) {
                                tags_vec.push(tag);
                            }
                        }
                    }

                    let record = Record::YomitanGlossary(Glossary {
                        popularity: arr.get(4).and_then(|v| v.as_i64()).unwrap_or(0),
                        tags: tags_vec,
                        content: content_list,
                    });

                    let stored_reading = if !reading.is_empty() && reading != headword {
                        Some(reading.to_string())
                    } else {
                        None
                    };

                    let stored = StoredRecord {
                        dictionary_id: dict_id,
                        record,
                        reading: stored_reading.clone(),
                    };

                    let json_val = serde_json::to_string(&stored)?;

                    // Insert Headword mapping
                    stmt.execute(rusqlite::params![headword, json_val])?;

                    // Insert Reading mapping (if different)
                    if let Some(r) = stored_reading {
                        stmt.execute(rusqlite::params![r, json_val])?;
                    }
                }
            }
        }
    }

    tx.commit()?;
    info!("💾 [Import] Database transaction committed.");

    Ok(format!("Imported '{}'", dict_name))
}
