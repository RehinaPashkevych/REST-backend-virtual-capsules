use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::http::{Status};
use rocket::response::status::Custom;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::capsules::{Capsule, CAPSULES};
use crate::items::{ITEMS};
use crate::contributors::{CONTRIBUTORS};
use chrono::Utc;
use chrono::DateTime;

#[derive(Serialize, Deserialize, Clone)]
pub struct MergeRecord {
    pub old_capsule1: CapsuleDetails,
    pub old_capsule2: CapsuleDetails,
    pub new_merged_capsule: CapsuleDetails,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CapsuleDetails {
    pub id: u32,
    pub contributor_id: u32,
    pub time_created: DateTime<Utc>,
    pub time_changed: DateTime<Utc>,
    pub description: String,
    pub name: String,
    pub item_ids: Option<Vec<u32>>, // Assuming item IDs are relevant for the capsule details
}

pub static MERGE_RECORDS: Lazy<Mutex<Vec<MergeRecord>>> = Lazy::new(|| Mutex::new(vec![]));

impl From<Capsule> for CapsuleDetails {
    fn from(capsule: Capsule) -> Self {
        CapsuleDetails {
            id: capsule.id,
            contributor_id: capsule.contributor_id,
            time_created: capsule.time_created,
            time_changed: capsule.time_changed.expect("REASON"),
            description: capsule.description,
            name: capsule.name,
            item_ids: capsule.item_ids,
        }
    }
}

#[post("/merges/<capsule_id1>/<capsule_id2>")]
pub fn merge_capsules(capsule_id1: u32, capsule_id2: u32) -> Result<Json<CapsuleDetails>, Custom<String>> {
    let mut capsules = CAPSULES.lock().unwrap();
    let idx1 = capsules.iter().position(|c| c.id == capsule_id1);
    let idx2 = capsules.iter().position(|c| c.id == capsule_id2);

    if idx1.is_none() || idx2.is_none() {
        return Err(Custom(Status::BadRequest, "One or both capsules not found.".into()));
    }

    let idx1 = idx1.unwrap();
    let idx2 = idx2.unwrap();

    if capsules[idx1].contributor_id != capsules[idx2].contributor_id {
        return Err(Custom(Status::BadRequest, "Capsules belong to different contributors.".into()));
    }

    let mut capsule1 = capsules[idx1].clone();
    let capsule2 = capsules[idx2].clone();

    if let Some(ref mut item_ids1) = capsule1.item_ids {
        let item_ids2 = capsule2.item_ids.clone().unwrap_or_else(Vec::new);
        item_ids1.extend(item_ids2);
    } else {
        capsule1.item_ids = capsule2.item_ids.clone();
    }

    let mut items = ITEMS.lock().unwrap();
    items.iter_mut().filter(|item| item.id_capsule == capsule_id2)
         .for_each(|item| item.id_capsule = capsule_id1);

    let mut contributors = CONTRIBUTORS.lock().unwrap();
    if let Some(contributor) = contributors.iter_mut().find(|c| c.id == capsule1.contributor_id) {
        if let Some(capsule_ids) = &mut contributor.capsule_ids {
            capsule_ids.retain(|&id| id != capsule_id2);
            if !capsule_ids.contains(&capsule_id1) {
                capsule_ids.push(capsule_id1);
            }
        }
    }

    capsules[idx1] = capsule1.clone();
    capsules.remove(idx2);

    let merge_record = MergeRecord {
        old_capsule1: CapsuleDetails::from(capsule1.clone()),
        old_capsule2: CapsuleDetails::from(capsule2),
        new_merged_capsule: CapsuleDetails::from(capsule1.clone()),
    };
    MERGE_RECORDS.lock().unwrap().push(merge_record);

    Ok(Json(CapsuleDetails::from(capsule1)))
}

#[get("/merges")]
pub fn list_merges() -> Json<Vec<MergeRecord>> {
    let merge_records = MERGE_RECORDS.lock().unwrap();
    Json(merge_records.clone())
}
