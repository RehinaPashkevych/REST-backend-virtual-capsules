use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::http::{Status};
use rocket::response::status::Custom;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::{DateTime, Utc};

use crate::capsules::{Capsule, CAPSULES};
use crate::contributors::CONTRIBUTORS;
use crate::items::ITEMS;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct MergeRecord {
    pub old_capsule1: CapsuleDetails,
    pub old_capsule2: CapsuleDetails,
    pub new_merged_capsule: CapsuleDetails,
}

pub static MERGE_RECORDS: Lazy<Mutex<Vec<MergeRecord>>> = Lazy::new(|| Mutex::new(vec![]));

impl From<Capsule> for CapsuleDetails {
    fn from(capsule: Capsule) -> Self {
        CapsuleDetails {
            id: capsule.id,
            contributor_id: capsule.contributor_id,
            time_created: capsule.time_created,
            time_changed: capsule.time_changed.expect("Time changed should be set"),
            description: capsule.description,
            name: capsule.name,
            item_ids: capsule.item_ids,
        }
    }
}

#[derive(Deserialize)]
struct MergeRequest {
    capsule_id1: u32,
    capsule_id2: u32,
}

#[post("/merges", format = "json", data = "<merge_request>")]
pub fn merge_capsules(merge_request: Json<MergeRequest>) -> Result<Json<CapsuleDetails>, Custom<String>> {
    let mut capsules = CAPSULES.lock().unwrap();
    let mut items = ITEMS.lock().unwrap();
    let mut contributors = CONTRIBUTORS.lock().unwrap();

    let idx1 = capsules.iter().position(|c| c.id == merge_request.capsule_id1);
    let idx2 = capsules.iter().position(|c| c.id == merge_request.capsule_id2);

    if idx1.is_none() || idx2.is_none() {
        return Err(Custom(Status::BadRequest, "One or both capsules not found.".into()));
    }

    let idx1 = idx1.unwrap();
    let idx2 = idx2.unwrap();

    if capsules[idx1].contributor_id != capsules[idx2].contributor_id {
        return Err(Custom(Status::Forbidden, "Capsules have different contributors.".into()));
    }

    let time_now = Utc::now();
    if capsules[idx1].time_changed.unwrap() > time_now || capsules[idx2].time_changed.unwrap() > time_now {
        return Err(Custom(Status::Forbidden, "Capsule modification not allowed at this time.".into()));
    }

    // Clone the old capsules for the record before any modification
    let old_capsule1 = capsules[idx1].clone().into();
    let old_capsule2 = capsules[idx2].clone().into();

    // Transfer all items from the second capsule to the first capsule
    let item_ids_from_capsule2 = capsules[idx2].item_ids.take().unwrap_or_default();
    for item_id in item_ids_from_capsule2.iter() {
        if let Some(item) = items.iter_mut().find(|i| i.id == *item_id) {
            item.id_capsule = capsules[idx1].id; // Update the capsule ID of the item
        }
    }

    if let Some(ref mut item_ids_from_capsule1) = capsules[idx1].item_ids {
        item_ids_from_capsule1.extend(item_ids_from_capsule2);
    } else {
        capsules[idx1].item_ids = Some(item_ids_from_capsule2);
    }

      // Update contributor's capsule list by removing the second capsule
      if let Some(contributor) = contributors.iter_mut().find(|c| c.id == capsules[idx1].contributor_id) {
        if let Some(capsule_ids) = &mut contributor.capsule_ids {
            capsule_ids.retain(|&id| id != capsules[idx2].id);
        }
    }


    // Remove the second capsule
    capsules.remove(idx2);

    // Create updated capsule details to return
    let updated_capsule = CapsuleDetails {
        id: capsules[idx1].id,
        contributor_id: capsules[idx1].contributor_id,
        time_created: capsules[idx1].time_created,
        time_changed: Utc::now(),
        description: format!("Updated by merging with Capsule {}", capsules[idx2].id),
        name: capsules[idx1].name.clone(),
        item_ids: capsules[idx1].item_ids.clone(),
    };

    // Store the merge record
    let merge_record = MergeRecord {
        old_capsule1: old_capsule1,
        old_capsule2: old_capsule2,
        new_merged_capsule: updated_capsule.clone(),
    };
    MERGE_RECORDS.lock().unwrap().push(merge_record);

    Ok(Json(updated_capsule))
}


#[get("/merges")]
pub fn get_merge_records() -> Json<Vec<MergeRecord>> {
    let merge_records = MERGE_RECORDS.lock().unwrap();
    Json(merge_records.clone())
}
