//! Integration: build a small PDF on the fly with `printpdf`, then exercise
//! `PdfExtractor` against the real file. Avoids checking large binaries in
//! while still using a real PDF parser end-to-end.

use adapter_extractor_pdf::PdfExtractor;
use librarian_domain::{ChunkPayload, ContentType, Document, Extractor, SourceHash, SourceId};
use printpdf::*;
use std::io::BufWriter;
use std::path::PathBuf;
use tempfile::tempdir;

fn write_pdf(path: &PathBuf, title: &str, pages: &[&[&str]]) {
    let (mut doc, page1, layer1) = PdfDocument::new(title, Mm(210.0), Mm(297.0), "L1");
    doc = doc.with_author("Test Author");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();

    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    for (page_idx, paragraphs) in pages.iter().enumerate() {
        if page_idx > 0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), format!("L{}", page_idx + 1));
            current_layer = doc.get_page(p).get_layer(l);
        }
        let mut y = Mm(280.0);
        for para in *paragraphs {
            current_layer.use_text(*para, 12.0, Mm(20.0), y, &font);
            y = Mm(y.0 - 8.0);
            // simulate paragraph break with a blank line
            y = Mm(y.0 - 8.0);
        }
    }

    let f = std::fs::File::create(path).unwrap();
    doc.save(&mut BufWriter::new(f)).unwrap();
}

fn doc(path: PathBuf, ct: ContentType) -> Document {
    Document {
        source_id: SourceId("s".into()),
        source_hash: SourceHash("h".into()),
        content_type: ct,
        path,
        work_id: None,
    }
}

#[test]
fn extracts_text_with_page_numbers_from_two_page_pdf() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("book.pdf");
    write_pdf(
        &path,
        "Hexagonal Architecture",
        &[
            &["Chapter introduction.", "Hexagons separate domain from adapters.", "Domain rules drive the bus."],
            &["Continued discussion.", "Each adapter implements a port.", "Adapters never see each other."],
        ],
    );

    let extracted = PdfExtractor.extract(&doc(path, ContentType::Book)).expect("extract");
    assert!(!extracted.spans.is_empty(), "got at least one span");
    // Every span carries a page number.
    assert!(extracted.spans.iter().all(|s| s.page.is_some()));
    // We have spans on both pages.
    let pages: std::collections::HashSet<_> = extracted.spans.iter().filter_map(|s| s.page).collect();
    assert!(pages.contains(&1) && pages.contains(&2), "spans cover both pages");
}

#[test]
fn book_payload_pulls_title_and_author_from_pdf_info() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("titled.pdf");
    write_pdf(&path, "A Real Title", &[&["body"]]);

    let payload = PdfExtractor.payload_for(&doc(path, ContentType::Book)).expect("payload");
    if let ChunkPayload::Book(meta) = payload {
        assert_eq!(meta.title, "A Real Title");
        assert_eq!(meta.author.as_deref(), Some("Test Author"));
    } else {
        panic!("expected Book payload");
    }
}

#[test]
fn paper_payload_has_authors_vector_populated() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("paper.pdf");
    write_pdf(&path, "On Vector DBs", &[&["abstract"]]);

    let payload = PdfExtractor.payload_for(&doc(path, ContentType::Paper)).expect("payload");
    if let ChunkPayload::Paper(meta) = payload {
        assert_eq!(meta.title, "On Vector DBs");
        assert_eq!(meta.authors, vec!["Test Author".to_string()]);
    } else {
        panic!("expected Paper payload");
    }
}
