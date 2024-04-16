use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::http::Status;
use rocket::response::status::Custom;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::capsules::{Capsule, CAPSULES};
use crate::items::ITEMS;
use crate::contributors::CONTRIBUTORS;

#[derive(Serialize, Deserialize, Clone)]
pub struct MergeRecord {
    pub old_capsule1: Capsule,
    pub old_capsule2: Capsule,
    pub new_merged_capsule: Capsule,
}


pub static MERGE_RECORDS: Lazy<Mutex<Vec<MergeRecord>>> = Lazy::new(|| Mutex::new(vec![]));



// update id capsule for items !!!!!!!!!!!!!!!!!!!!!!!!!!!

#[post("/merges/<capsule_id1>/<capsule_id2>")]
pub fn merge_capsules(capsule_id1: u32, capsule_id2: u32) -> Result<Json<Capsule>, Custom<String>> {
    let mut capsules = CAPSULES.lock().unwrap();
    let mut contributors = CONTRIBUTORS.lock().unwrap();

    // Get indices of the capsules to merge
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

    // Merge items
    if let Some(ref mut item_ids1) = capsule1.item_ids {
        if let Some(item_ids2) = &capsule2.item_ids {
            item_ids1.extend(item_ids2.clone()); // Use clone here to avoid moving capsule2.item_ids
        }
    } else {
        capsule1.item_ids = capsule2.item_ids.clone(); // Clone the item_ids for assignment
    }

    // Update the contributor's list of capsules
    if let Some(contributor) = contributors.iter_mut().find(|c| c.id == capsule1.contributor_id) {
        if let Some(capsule_ids) = &mut contributor.capsule_ids {
            capsule_ids.retain(|&id| id != capsule_id2); // Remove the ID of the second capsule
            if !capsule_ids.contains(&capsule_id1) {
                capsule_ids.push(capsule_id1); // Ensure the first capsule's ID is still present
            }
        } else {
            // Initialize if None
            contributor.capsule_ids = Some(vec![capsule_id1]);
        }
    }

    // Perform updates on the global capsules list
    capsules[idx1] = capsule1.clone(); // Update the first capsule
    capsules.remove(idx2); // Remove the second capsule

    // Create and save the merge record
    let merge_record = MergeRecord {
        old_capsule1: capsule1.clone(),
        old_capsule2: capsule2,
        new_merged_capsule: capsule1.clone(),
    };

    let mut merge_records = MERGE_RECORDS.lock().unwrap();
    merge_records.push(merge_record);

    Ok(Json(capsule1))
}
