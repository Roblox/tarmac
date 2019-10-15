use std::path::Path;

use crate::auth_cookie::get_auth_cookie;

pub fn upload_image(path: &Path) {
    // let cookie = get_auth_cookie().expect("no auth cookie");
    // let url = format!(
    //     "https://data.roblox.com/Data/Upload.ashx?assetid=0",
    //     asset_id
    // );

    // let buffer = fs::read(path).expect("couldn't read file");

    // let client = reqwest::Client::new();
    // let response = client
    //     .get(&url)
    //     .header(COOKIE, format!(".ROBLOSECURITY={}", auth_cookie))
    //     .header(CONTENT_TYPE, "application/xml")
    //     .header(USER_AGENT, "Roblox/WinInet")
    //     .body(buffer)
    //     .send()
    //     .map_err(rlua::Error::external)?;

    // if response.status().is_success() {
    //     Ok(())
    // } else {
    //     Err(rlua::Error::external(format!(
    //         "Roblox API returned an error, status {}.",
    //         response.status()
    //     )))
    // }
}
