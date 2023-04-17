use anyhow::Result;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PhotosPayload {
    pub photos: Photos,
}

#[derive(Deserialize, Debug)]
pub struct Photos {
    pub photo: Vec<SimplePhoto>,
}

#[derive(Deserialize, Debug)]
pub struct SimplePhoto {
    pub id: String,
    pub owner: String,
    pub title: String,
    pub farm: u64,
    pub server: String,
    pub secret: String,
}

pub struct FlickrClient {
    http: ClientWithMiddleware,
    api_key: String,
}

impl FlickrClient {
    pub fn new(api_key: &str) -> Self {
        let http = ClientBuilder::new(reqwest::Client::new())
            .with(Cache(HttpCache {
                mode: CacheMode::OnlyIfCached,
                manager: CACacheManager::default(),
                options: None,
            }))
            .build();

        Self {
            http,
            api_key: api_key.into(),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SimplePhoto>> {
        let url = format!("https://www.flickr.com/services/rest/?method=flickr.photos.search&api_key={}&text={}&sort=relevance&per_page=10&media=photos&format=json&nojsoncallback=1", self.api_key, query);
        let response = self.http.get(url).send().await?;
        let payload = response.json::<PhotosPayload>().await?;

        Ok(payload.photos.photo)
    }

    pub async fn image(&self, photo: &SimplePhoto) -> Result<Vec<u8>> {
        let url = format!(
            "https://farm{}.static.flickr.com/{}/{}_{}.jpg",
            photo.farm, photo.server, photo.id, photo.secret
        );

        let response = self.http.get(url).send().await?;

        Ok(response.bytes().await?.into())
    }
}
