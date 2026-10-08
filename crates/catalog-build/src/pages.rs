use askama::Template;
use catalog_model::{CatalogObject, NavigationMode, ObjectDetail, ObjectKind, RouteType};
use pulldown_cmark::{Event, Parser, Tag, TagEnd, html};
use url::Url;

/// Check if a link destination is safe.
///
/// Allows:
/// - credential-free http:// and https:// URLs
/// - validated root-relative paths starting with '/' (excluding protocol-relative '//')
///
/// Rejects:
/// - javascript:, data:, vbscript: schemes
/// - protocol-relative '//' URLs
/// - credentials in authority (e.g. http://user:pass@host)
pub fn is_safe_link(url: &str) -> bool {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
    {
        return false;
    }

    if trimmed.starts_with("//") {
        return false;
    }

    if trimmed.starts_with('/') {
        return true;
    }

    if let Ok(parsed) = Url::parse(trimmed)
        && (parsed.scheme() == "http" || parsed.scheme() == "https")
        && parsed.username().is_empty()
        && parsed.password().is_none()
    {
        return true;
    }

    false
}

/// Render Markdown to safe HTML.
/// - Drops raw HTML events completely.
/// - Drops Markdown image events (photos use the typed photo pipeline).
/// - Allows only safe HTTP/HTTPS links and root-relative URLs; replaces unsafe link destinations with '#'.
/// - Escapes fenced code blocks as text.
pub fn render_markdown(input: &str) -> String {
    let parser = Parser::new(input);
    let mut safe_events = Vec::new();
    let mut drop_image_depth = 0;

    for event in parser {
        match event {
            Event::Html(_) | Event::InlineHtml(_) => {
                // Drop raw HTML
            }
            Event::Start(Tag::Image { .. }) => {
                drop_image_depth += 1;
            }
            Event::End(TagEnd::Image) => {
                if drop_image_depth > 0 {
                    drop_image_depth -= 1;
                }
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                if drop_image_depth > 0 {
                    continue;
                }
                if is_safe_link(&dest_url) {
                    safe_events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }));
                } else {
                    safe_events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url: "#".into(),
                        title,
                        id,
                    }));
                }
            }
            other => {
                if drop_image_depth == 0 {
                    safe_events.push(other);
                }
            }
        }
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, safe_events.into_iter());
    html_output
}

#[derive(Template)]
#[template(path = "route.html")]
pub struct RoutePageTemplate<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub canonical_url: &'a str,
    pub detail: &'a ObjectDetail,
    pub note_html: String,
    pub unverified_preview: bool,
}

#[derive(Template)]
#[template(path = "road.html")]
pub struct RoadPageTemplate<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub canonical_url: &'a str,
    pub detail: &'a ObjectDetail,
    pub unverified_preview: bool,
}

#[derive(Template)]
#[template(path = "place.html")]
pub struct PlacePageTemplate<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub canonical_url: &'a str,
    pub detail: &'a ObjectDetail,
    pub note_html: String,
    pub unverified_preview: bool,
}

pub fn render_route_page(detail: &ObjectDetail, base_url: &str) -> Result<String, askama::Error> {
    render_route_page_with_preview(detail, base_url, false)
}

pub fn render_route_page_with_preview(
    detail: &ObjectDetail,
    base_url: &str,
    unverified_preview: bool,
) -> Result<String, askama::Error> {
    let note_html = if let Some(note) = &detail.author_note {
        render_markdown(&note.text)
    } else {
        String::new()
    };

    let trimmed_base = base_url.trim_end_matches('/');
    let canonical_url = format!("{}/routes/{}/index.html", trimmed_base, detail.key.id);

    let template = RoutePageTemplate {
        title: &detail.title,
        description: &detail.summary,
        canonical_url: &canonical_url,
        detail,
        note_html,
        unverified_preview,
    };

    template.render()
}

pub fn render_road_page(detail: &ObjectDetail, base_url: &str) -> Result<String, askama::Error> {
    render_road_page_with_preview(detail, base_url, false)
}

pub fn render_road_page_with_preview(
    detail: &ObjectDetail,
    base_url: &str,
    unverified_preview: bool,
) -> Result<String, askama::Error> {
    let trimmed_base = base_url.trim_end_matches('/');
    let canonical_url = format!("{}/roads/{}/index.html", trimmed_base, detail.key.id);

    let template = RoadPageTemplate {
        title: &detail.title,
        description: &detail.summary,
        canonical_url: &canonical_url,
        detail,
        unverified_preview,
    };

    template.render()
}

pub fn render_place_page(detail: &ObjectDetail, base_url: &str) -> Result<String, askama::Error> {
    render_place_page_with_preview(detail, base_url, false)
}

pub fn render_place_page_with_preview(
    detail: &ObjectDetail,
    base_url: &str,
    unverified_preview: bool,
) -> Result<String, askama::Error> {
    let note_html = if let Some(note) = &detail.author_note {
        render_markdown(&note.text)
    } else {
        String::new()
    };

    let trimmed_base = base_url.trim_end_matches('/');
    let canonical_url = format!("{}/places/{}/index.html", trimmed_base, detail.key.id);

    let template = PlacePageTemplate {
        title: &detail.title,
        description: &detail.summary,
        canonical_url: &canonical_url,
        detail,
        note_html,
        unverified_preview,
    };

    template.render()
}

#[derive(Template)]
#[template(path = "root_map.html")]
pub struct RootMapTemplate<'a> {
    pub head_assets: &'a str,
    pub unverified_preview: bool,
    pub config_json: &'a str,
}

pub fn render_root_map(
    head_assets: &str,
    unverified_preview: bool,
    config_json: &str,
) -> Result<String, askama::Error> {
    let template = RootMapTemplate {
        head_assets,
        unverified_preview,
        config_json,
    };
    template.render()
}

#[derive(Template)]
#[template(path = "privacy.html")]
pub struct PrivacyPageTemplate<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub canonical_url: &'a str,
    pub production_url: &'a str,
    pub unverified_preview: bool,
}

pub fn render_privacy_page(
    production_url: &str,
    unverified_preview: bool,
) -> Result<String, askama::Error> {
    let trimmed_base = production_url.trim_end_matches('/');
    let canonical_url = format!("{}/privacy/index.html", trimmed_base);
    let template = PrivacyPageTemplate {
        title: "Privacy Policy",
        description: "Privacy policy and analytics disclosure for Ride Atlas.",
        canonical_url: &canonical_url,
        production_url: trimmed_base,
        unverified_preview,
    };
    template.render()
}

/// Generate sitemap.xml for all catalog objects.
/// Uses authored `updated_on` only; omits lastmod when no date is supplied.
pub fn generate_sitemap(base_url: &str, objects: &[CatalogObject]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    let trimmed_base = base_url.trim_end_matches('/');

    out.push_str(&format!("  <url><loc>{}/</loc></url>\n", trimmed_base));
    out.push_str(&format!(
        "  <url><loc>{}/privacy/index.html</loc></url>\n",
        trimmed_base
    ));

    for obj in objects {
        let kind_dir = match obj.key.kind {
            ObjectKind::Route => "routes",
            ObjectKind::Road => "roads",
            ObjectKind::Place => "places",
        };
        let page_url = format!("{}/{}/{}/index.html", trimmed_base, kind_dir, obj.key.id);
        out.push_str("  <url>\n");
        out.push_str(&format!("    <loc>{}</loc>\n", page_url));
        if let Some(updated_on) = &obj.updated_on {
            out.push_str(&format!("    <lastmod>{}</lastmod>\n", updated_on));
        }
        out.push_str("  </url>\n");
    }
    out.push_str("</urlset>\n");
    out
}
