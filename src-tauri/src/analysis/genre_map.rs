/// Maps external genre/style keywords to Rekordbox My Tag genre names.
pub fn score_genre_tag(
    tag_name: &str,
    corpus: &str,
    genre_field: &str,
    path_folders: &str,
) -> (f64, String) {
    let keywords = genre_keywords(tag_name);
    let mut score = 0.0f64;
    let mut reasons = Vec::new();

    if !genre_field.is_empty() {
        for (kw, weight) in &keywords {
            if genre_field.contains(kw) {
                score = score.max(*weight);
                reasons.push(format!("genre field matches '{kw}'"));
            }
        }
        // Direct match on normalized genre field
        let tag_lower = tag_name.to_lowercase();
        if genre_field.contains(&tag_lower) || tag_lower.contains(genre_field) {
            score = score.max(0.9);
            reasons.push(format!("genre field is '{genre_field}'"));
        }
    }

    for (kw, weight) in &keywords {
        if corpus.contains(kw) || path_folders.contains(kw) {
            let w = *weight * 0.85;
            if w > score {
                score = w;
                reasons.push(format!("keyword '{kw}' in metadata/path"));
            } else if w >= score * 0.9 {
                reasons.push(format!("also '{kw}'"));
            }
        }
    }

    let reason = if reasons.is_empty() {
        String::new()
    } else {
        reasons.join("; ")
    };

    (score, reason)
}

fn genre_keywords(tag: &str) -> Vec<(&'static str, f64)> {
    match tag {
        "House" => vec![
            ("house", 0.92),
            ("tech house", 0.88),
            ("deep house", 0.88),
            ("afro house", 0.9),
            ("progressive house", 0.85),
            ("electro house", 0.82),
            ("funky house", 0.85),
            ("jackin", 0.75),
        ],
        "Classic House" => vec![
            ("classic house", 0.95),
            ("classic-house", 0.95),
            ("old skool house", 0.92),
            ("old school house", 0.92),
            ("piano house", 0.9),
            ("90s house", 0.88),
            ("rave classic", 0.86),
        ],
        "Deep House" => vec![("deep house", 0.94), ("deep-house", 0.94)],
        "Tech House" => vec![("tech house", 0.94), ("tech-house", 0.94)],
        "Acid House" => vec![("acid house", 0.94), ("acid-house", 0.94), ("303", 0.82)],
        "Electro House" => vec![("electro house", 0.94), ("electro-house", 0.94)],
        "Progressive House" => vec![
            ("progressive house", 0.94),
            ("prog house", 0.88),
        ],
        "Latin House" => vec![("latin house", 0.94)],
        "Afro House" => vec![("afro house", 0.94)],
        "Hip House" => vec![("hip house", 0.94), ("hip-house", 0.94)],
        "Techno" => vec![
            ("techno", 0.92),
            ("minimal techno", 0.9),
            ("industrial", 0.78),
            ("hard techno", 0.88),
            ("melodic techno", 0.86),
            ("peak time techno", 0.85),
        ],
        "Hip-Hop" => vec![
            ("hip hop", 0.92),
            ("hip-hop", 0.92),
            ("rap", 0.85),
            ("trap", 0.82),
            ("boom bap", 0.88),
            ("drill", 0.8),
        ],
        "Latin" => vec![
            ("latin", 0.9),
            ("reggaeton", 0.92),
            ("salsa", 0.88),
            ("bachata", 0.88),
            ("dembow", 0.85),
            ("cumbia", 0.85),
            ("merengue", 0.85),
        ],
        "Afro" => vec![
            ("afro", 0.9),
            ("afrobeats", 0.92),
            ("amapiano", 0.9),
            ("afrobeat", 0.9),
            ("gqom", 0.85),
        ],
        "Arabic" => vec![
            ("arabic", 0.92),
            ("khaleeji", 0.9),
            ("shaabi", 0.88),
            ("dabke", 0.85),
            ("middle east", 0.82),
        ],
        "Pop" => vec![
            ("pop", 0.88),
            ("top 40", 0.85),
            ("mainstream", 0.75),
            ("radio edit", 0.7),
        ],
        "R&B" => vec![
            ("r&b", 0.92),
            ("rnb", 0.92),
            ("rhythm and blues", 0.9),
            ("neo soul", 0.85),
            ("contemporary r&b", 0.88),
        ],
        "Disco" => vec![
            ("disco", 0.92),
            ("nu disco", 0.9),
            ("boogie", 0.82),
            ("funk", 0.75),
        ],
        "DnB" => vec![
            ("drum and bass", 0.92),
            ("drum & bass", 0.92),
            ("dnb", 0.9),
            ("jungle", 0.88),
            ("neurofunk", 0.85),
            ("liquid dnb", 0.85),
        ],
        "Breakbeat" => vec![
            ("breakbeat", 0.94),
            ("breakbeats", 0.92),
            ("breaks", 0.88),
            ("big beat", 0.9),
            ("bigbeat", 0.9),
            ("nu skool breaks", 0.86),
        ],
        "Electronic" => vec![
            ("electronic", 0.85),
            ("edm", 0.82),
            ("trance", 0.85),
            ("dubstep", 0.85),
            ("garage", 0.78),
            ("idm", 0.75),
        ],
        "Soul" => vec![
            ("soul", 0.9),
            ("motown", 0.85),
            ("gospel", 0.78),
        ],
        "Desi" => vec![
            ("desi", 0.9),
            ("bollywood", 0.92),
            ("bhangra", 0.88),
            ("punjabi", 0.85),
            ("hindi", 0.82),
        ],
        "East-Asian" => vec![
            ("k-pop", 0.92),
            ("kpop", 0.92),
            ("j-pop", 0.9),
            ("jpop", 0.9),
            ("city pop", 0.85),
            ("mandopop", 0.85),
            ("c-pop", 0.85),
        ],
        _ => vec![],
    }
}
