//! Immutable Pandora catalog admission tests.

use room_core::slice::{Invocation, PandoraText, RequestPages, SliceError};

fn requests() -> Vec<RequestPages> {
    Invocation::ALL
        .iter()
        .map(|i| i.source())
        .chain([0x88_b722, 0x88_b7e3])
        .fold(Vec::new(), |mut pages, source| {
            if !pages.iter().any(|r: &RequestPages| r.source == source) {
                let (count, choice) = Invocation::request_shape(source).unwrap();
                pages.push(RequestPages {
                    source,
                    pages: (0..count).map(|n| source * 16 + u32::from(n)).collect(),
                    choice,
                });
            }
            pages
        })
}
#[test]
fn exact_requests_and_repeated_invocations_are_distinct() {
    let text = PandoraText::new(requests()).unwrap();
    assert_ne!(Invocation::TourLeave41, Invocation::TourLeave44);
    assert_eq!(
        Invocation::TourLeave41.source(),
        Invocation::TourLeave44.source()
    );
    assert_eq!(text.pages(Invocation::BoxWarning).len(), 2);
}
#[test]
fn malformed_catalogs_fail_closed() {
    let mut pages = requests();
    pages[0].pages.pop();
    assert!(matches!(PandoraText::new(pages), Err(SliceError::Data)));
    let mut pages = requests();
    pages[1].pages[0] = pages[0].pages[0];
    assert!(PandoraText::new(pages).is_err());
    let mut pages = requests();
    pages[0].choice = false;
    assert!(PandoraText::new(pages).is_err());
    let mut pages = requests();
    pages.swap(0, 1);
    assert!(PandoraText::new(pages).is_err());
}
