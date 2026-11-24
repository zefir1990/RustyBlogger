pub fn language_country_code(language: &str) -> &'static str {
    match language {
        "ru" => "ru",
        "en" => "gb",
        "zh" => "cn",
        "de" => "de",
        "ja" => "jp",
        "fr" => "fr",
        "es" => "es",
        "pt" => "pt",
        "hi" => "in",
        "uk" => "ua",
        "it" => "it",
        "ko" => "kr",
        "ar" => "sa",
        "tr" => "tr",
        "pl" => "pl",
        "nl" => "nl",
        _ => "un",
    }
}

pub fn flag_svg(country_code: &str) -> Option<&'static str> {
    match country_code {
        "ru" => Some(include_str!("assets/flags/ru.svg")),
        "gb" => Some(include_str!("assets/flags/gb.svg")),
        "cn" => Some(include_str!("assets/flags/cn.svg")),
        "de" => Some(include_str!("assets/flags/de.svg")),
        "jp" => Some(include_str!("assets/flags/jp.svg")),
        "fr" => Some(include_str!("assets/flags/fr.svg")),
        "es" => Some(include_str!("assets/flags/es.svg")),
        "pt" => Some(include_str!("assets/flags/pt.svg")),
        "in" => Some(include_str!("assets/flags/in.svg")),
        "ua" => Some(include_str!("assets/flags/ua.svg")),
        "it" => Some(include_str!("assets/flags/it.svg")),
        "kr" => Some(include_str!("assets/flags/kr.svg")),
        "sa" => Some(include_str!("assets/flags/sa.svg")),
        "tr" => Some(include_str!("assets/flags/tr.svg")),
        "pl" => Some(include_str!("assets/flags/pl.svg")),
        "nl" => Some(include_str!("assets/flags/nl.svg")),
        "un" => Some(include_str!("assets/flags/un.svg")),
        _ => None,
    }
}
