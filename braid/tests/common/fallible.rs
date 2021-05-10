use crate::{Validated, ValidatedBuf};
use quickcheck_macros::quickcheck;
use static_assertions::{assert_eq_align, assert_eq_size, assert_eq_size_ptr, assert_eq_size_val};
use std::collections::HashSet;

#[test]
pub fn equality_tests() -> Result<(), Box<dyn std::error::Error>> {
    let x = ValidatedBuf::new("One")?;
    let y = Validated::from_str("One")?;
    assert_eq!(x, y);
    assert_eq!(x, *y);
    assert_eq!(&x, y);
    assert_eq!(y, x);
    assert_eq!(y, &x);
    assert_eq!(*y, x);

    assert_eq!("One", x.clone().into_string());
    let z = x.clone().into_boxed_ref();
    assert_eq!(y, z);
    assert_eq!(z, y);
    assert_eq!(x, z);
    assert_eq!(z, x);

    assert_eq!(x, z.into_owned());

    Ok(())
}

#[test]
fn debug_and_display_tests() -> Result<(), Box<dyn std::error::Error>> {
    let x = ValidatedBuf::new("One")?;
    let y = Validated::from_str("One")?;

    assert_eq!("One", x.to_string());
    assert_eq!("One", y.to_string());
    assert_eq!("ValidatedBuf(\"One\")", format!("{:?}", x));
    assert_eq!("Validated(\"One\")", format!("{:?}", y));

    Ok(())
}

#[quickcheck]
fn owned_and_borrowed_hashes_are_equivalent(s: String) -> quickcheck::TestResult {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let owned = if let Ok(x) = ValidatedBuf::new(s.clone()) {
        x
    } else {
        return quickcheck::TestResult::discard();
    };

    let owned_hash = {
        let mut hasher = DefaultHasher::new();
        owned.hash(&mut hasher);
        hasher.finish()
    };

    let borrowed = Validated::from_str(&s).unwrap();

    let borrowed_hash = {
        let mut hasher = DefaultHasher::new();
        borrowed.hash(&mut hasher);
        hasher.finish()
    };

    if owned_hash == borrowed_hash {
        quickcheck::TestResult::passed()
    } else {
        quickcheck::TestResult::failed()
    }
}

#[test]
fn can_use_as_hash_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = HashSet::new();

    assert!(map.insert(ValidatedBuf::new("One")?));
    assert!(map.insert(ValidatedBuf::new("Seven")?));

    assert!(map.contains(Validated::from_str("One")?));
    assert!(map.contains(&ValidatedBuf::new("One")?));
    assert!(!map.contains(Validated::from_str("Two")?));

    assert!(!map.remove(Validated::from_str("Two")?));
    assert!(map.remove(Validated::from_str("One")?));
    assert!(!map.remove(Validated::from_str("One")?));

    assert!(map.remove(&ValidatedBuf::new("Seven")?));
    assert!(!map.remove(Validated::from_str("Seven")?));

    assert!(map.is_empty());

    Ok(())
}

#[test]
fn can_use_refs_as_hash_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = HashSet::new();

    assert!(map.insert(Validated::from_str("One")?));
    assert!(map.insert(Validated::from_str("Seven")?));

    assert!(map.contains(Validated::from_str("One")?));
    assert!(map.contains(&*ValidatedBuf::new("One")?));
    assert!(!map.contains(Validated::from_str("Two")?));

    assert!(!map.remove(Validated::from_str("Two")?));
    assert!(map.remove(Validated::from_str("One")?));
    assert!(!map.remove(Validated::from_str("One")?));

    assert!(map.remove(&*ValidatedBuf::new("Seven")?));
    assert!(!map.remove(Validated::from_str("Seven")?));

    assert!(map.is_empty());

    Ok(())
}

#[test]
#[should_panic]
fn verify_serialization_fail_borrow() {
    const SERIALIZATION: &str = "\"Test 🏗\"";
    dbg!(SERIALIZATION.as_bytes());
    let _: &Validated = serde_json::from_str(&SERIALIZATION).unwrap();
}

#[test]
#[should_panic]
fn verify_serialization_fail_boxed() {
    const SERIALIZATION: &str = "\"Test 🏗\"";
    let _: Box<Validated> = serde_json::from_str(&SERIALIZATION).unwrap();
}

#[test]
#[should_panic]
fn verify_serialization_fail_owned() {
    const SERIALIZATION: &str = "\"Test 🏗\"";
    let _: ValidatedBuf = serde_json::from_str(&SERIALIZATION).unwrap();
}

#[test]
fn verify_serialization_pass_borrow() -> Result<(), Box<dyn std::error::Error>> {
    const SERIALIZATION: &str = "\"Test ;\"";
    let expected = Validated::from_str("Test ;")?;
    let actual: &Validated = serde_json::from_str(&SERIALIZATION)?;
    assert_eq!(expected, actual);
    Ok(())
}

#[test]
fn verify_serialization_pass_boxed() -> Result<(), Box<dyn std::error::Error>> {
    const SERIALIZATION: &str = "\"Test ;\"";
    let expected = Validated::from_str("Test ;")?;
    let actual: Box<Validated> = serde_json::from_str(&SERIALIZATION)?;
    assert_eq!(expected, actual);
    Ok(())
}

#[test]
fn verify_serialization_pass_owned() -> Result<(), Box<dyn std::error::Error>> {
    const SERIALIZATION: &str = "\"Test ;\"";
    let expected = Validated::from_str("Test ;")?;
    let actual: ValidatedBuf = serde_json::from_str(&SERIALIZATION)?;
    assert_eq!(expected, actual);
    Ok(())
}

#[test]
fn check_reference_alignment() {
    dbg!(std::mem::align_of::<&str>());
    dbg!(std::mem::align_of::<&Validated>());
    assert_eq_align!(&Validated, &str);
}

#[test]
fn check_reference_size() {
    dbg!(std::mem::size_of::<&str>());
    dbg!(std::mem::size_of::<&Validated>());
    assert_eq_size!(&Validated, &str);
}

#[test]
fn check_reference_size_ptr() {
    let s = "source";
    let y: &Validated = Validated::from_str(s).unwrap();
    assert_eq_size_ptr!(&s, &y);
}

#[test]
fn check_reference_size_val() {
    let s = "source";
    let y: &Validated = Validated::from_str(s).unwrap();
    dbg!(std::mem::size_of_val(s));
    dbg!(std::mem::size_of_val(y));
    assert_eq_size_val!(s, y);
}

#[test]
fn check_boxed_ref_alignment() {
    dbg!(std::mem::align_of::<Box<str>>());
    dbg!(std::mem::align_of::<Box<Validated>>());
    assert_eq_align!(Box<Validated>, Box<str>);
}

#[test]
fn check_boxed_ref_size() {
    dbg!(std::mem::size_of::<Box<str>>());
    dbg!(std::mem::size_of::<Box<Validated>>());
    assert_eq_size!(Box<Validated>, Box<str>);
}

#[test]
fn check_boxed_ref_size_ptr() {
    let source = String::from("source");
    let s = source.clone().into_boxed_str();
    let y: Box<Validated> = ValidatedBuf::new(source).unwrap().into_boxed_ref();
    assert_eq_size_ptr!(&s, &y);
}

#[test]
fn check_boxed_ref_size_val() {
    let source = String::from("source");
    let s = source.clone().into_boxed_str();
    let y: Box<Validated> = ValidatedBuf::new(source).unwrap().into_boxed_ref();
    dbg!(std::mem::size_of_val(&s));
    dbg!(std::mem::size_of_val(&y));
    assert_eq_size_val!(s, y);
}

#[test]
fn check_owned_alignment() {
    dbg!(std::mem::align_of::<String>());
    dbg!(std::mem::align_of::<ValidatedBuf>());
    assert_eq_align!(ValidatedBuf, String);
}

#[test]
fn check_owned_size() {
    dbg!(std::mem::size_of::<String>());
    dbg!(std::mem::size_of::<ValidatedBuf>());
    assert_eq_size!(ValidatedBuf, String);
}

#[test]
fn check_owned_size_ptr() {
    let s = String::from("source");
    let y: ValidatedBuf = ValidatedBuf::new(s.clone()).unwrap();
    assert_eq_size_ptr!(&s, &y);
}

#[test]
fn check_owned_size_val() {
    let s = String::from("source");
    let y: ValidatedBuf = ValidatedBuf::new(s.clone()).unwrap();
    dbg!(std::mem::size_of_val(&s));
    dbg!(std::mem::size_of_val(&y));
    assert_eq_size_val!(s, y);
}
