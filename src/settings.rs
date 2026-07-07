use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {

}

pub struct OpenAI {
    api_key: String,
    
}