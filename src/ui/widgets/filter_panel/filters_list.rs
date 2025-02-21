use crate::client::structs::FilterItem;

#[derive(Default)]
pub struct FiltersList {
    pub playback_status: u32,
    pub favourite: bool,
    pub genres: Option<Vec<FilterItem>>,
    pub tags: Option<Vec<FilterItem>>,
    pub years: Option<Vec<FilterItem>>,
    pub ratings: Option<Vec<FilterItem>>,
    pub studios: Option<Vec<FilterItem>>,
    pub containers: Option<Vec<FilterItem>>,
    pub encoders: Option<Vec<FilterItem>>,
    pub video_types: Option<Vec<FilterItem>>,
    pub resolution: u32,
}

impl FiltersList {
    pub fn to_kv(&self) -> Vec<(String, String)> {
        let mut kv = Vec::new();

        let mut filters_kv = Vec::new();
        match self.playback_status {
            1 => filters_kv.push("IsPlayed".to_owned()),
            2 => filters_kv.push("IsUnplayed".to_owned()),
            3 => filters_kv.push("IsResumable".to_owned()),
            _ => (),
        }
        if self.favourite {
            filters_kv.push("IsFavorite".to_owned());
        }
        if !filters_kv.is_empty() {
            kv.push(("filters".to_owned(), filters_kv.join(",")));
        }

        if let Some(genres) = &self.genres {
            let genre_ids: Vec<_> = genres.iter().filter_map(|f| f.id.to_owned()).collect();

            if !genre_ids.is_empty() {
                kv.push(("GenreIds".to_owned(), genre_ids.join(",")));
            }
        }

        if let Some(tags) = &self.tags {
            let tag_ids: Vec<_> = tags.iter().filter_map(|f| f.id.to_owned()).collect();

            if !tag_ids.is_empty() {
                kv.push(("TagIds".to_owned(), tag_ids.join(",")));
            }
        }

        if let Some(years) = &self.years {
            kv.push((
                "Years".to_owned(),
                years
                    .iter()
                    .map(|f| f.name.to_owned())
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }

        if let Some(ratings) = &self.ratings {
            kv.push((
                "OfficialRatings".to_owned(),
                ratings
                    .iter()
                    .map(|f| f.name.to_owned())
                    .collect::<Vec<_>>()
                    .join("|"),
            ));
        }

        if let Some(studios) = &self.studios {
            let studio_ids: Vec<_> = studios.iter().filter_map(|f| f.id.to_owned()).collect();

            if !studio_ids.is_empty() {
                kv.push(("StudioIds".to_owned(), studio_ids.join(",")));
            }
        }

        if let Some(containers) = &self.containers {
            kv.push((
                "Containers".to_owned(),
                containers
                    .iter()
                    .map(|f| f.name.to_owned())
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }

        if let Some(encoders) = &self.encoders {
            kv.push((
                "VideoCodecs".to_owned(),
                encoders
                    .iter()
                    .map(|f| f.name.to_owned())
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }

        if let Some(video_types) = &self.video_types {
            let video_type_ids: Vec<_> =
                video_types.iter().filter_map(|f| f.id.to_owned()).collect();

            if !video_type_ids.is_empty() {
                kv.push(("ExtendedVideoTypes".to_owned(), video_type_ids.join(",")));
            }
        }

        match self.resolution {
            1 => kv.push(("MinWidth".to_owned(), "3800".to_owned())),
            2 => {
                kv.push(("MinWidth".to_owned(), "1800".to_owned()));
                kv.push(("MaxWidth".to_owned(), "2200".to_owned()));
            }
            3 => {
                kv.push(("MinWidth".to_owned(), "1200".to_owned()));
                kv.push(("MaxWidth".to_owned(), "1799".to_owned()));
            }
            4 => kv.push(("MaxWidth".to_owned(), "1199".to_owned())),
            _ => (),
        }

        kv
    }

    pub fn is_empty(&self) -> bool {
        self.playback_status == 0
            && !self.favourite
            && self.genres.as_ref().is_none_or(|v| v.is_empty())
            && self.tags.as_ref().is_none_or(|v| v.is_empty())
            && self.years.as_ref().is_none_or(|v| v.is_empty())
            && self.ratings.as_ref().is_none_or(|v| v.is_empty())
            && self.studios.as_ref().is_none_or(|v| v.is_empty())
            && self.containers.as_ref().is_none_or(|v| v.is_empty())
            && self.encoders.as_ref().is_none_or(|v| v.is_empty())
            && self.video_types.as_ref().is_none_or(|v| v.is_empty())
            && self.resolution == 0
    }
}
