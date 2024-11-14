use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Hash, PartialEq, Eq, Deserialize, Serialize)]
enum ItemType {
    Character,
    Backpack,
    Pickaxe,
    Glider,
    SkyDiveContrail,
    Dance,
    ItemWrap,
    BannerIcon,
    BannerColor,
    MusicPack,
    LoadingScreen,
    MiscItem,
}

#[derive(Deserialize)]
struct ActiveItems {
    items: HashMap<ItemType, Vec<String>>,
}

struct ItemManager {
    active_items: ActiveItems,
}

impl ItemManager {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let file = fs::read_to_string("./active_items.json")?;
        let active_items = serde_json::from_str(&file)?;
        Ok(ItemManager { active_items })
    }

    fn get_item_type(&self, item_id: &str) -> Option<&ItemType> {
        self.active_items.items.iter().find_map(|(item_type, ids)| {
            if ids.contains(&item_id.to_string()) {
                Some(item_type)
            } else {
                None
            }
        })
    }
}
