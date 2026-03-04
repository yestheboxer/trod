use trod::db::Database;
use std::path::Path;
use tempfile::NamedTempFile;

fn test_db() -> Database {
    let tmp = NamedTempFile::new().unwrap();
    Database::open(tmp.path()).unwrap()
}

#[test]
fn test_add_directory() {
    let db = test_db();
    db.add("/home/user/projects").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].path, "/home/user/projects");
    assert_eq!(dirs[0].visit_count, 1);
}

#[test]
fn test_add_directory_twice_increments_count() {
    let db = test_db();
    db.add("/home/user/projects").unwrap();
    db.add("/home/user/projects").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].visit_count, 2);
}

#[test]
fn test_list_recent_ordering() {
    let db = test_db();
    db.add("/first").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.add("/second").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs[0].path, "/second");
    assert_eq!(dirs[1].path, "/first");
}

#[test]
fn test_forget_directory() {
    let db = test_db();
    db.add("/home/user/projects").unwrap();
    db.forget("/home/user/projects").unwrap();

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn test_list_frequent_ordering() {
    let db = test_db();
    db.add("/rare").unwrap();
    db.add("/common").unwrap();
    db.add("/common").unwrap();
    db.add("/common").unwrap();

    let dirs = db.list_frequent(10).unwrap();
    assert_eq!(dirs[0].path, "/common");
    assert_eq!(dirs[1].path, "/rare");
}

#[test]
fn test_clean_removes_nonexistent() {
    let db = test_db();
    db.add("/this/path/does/not/exist/at/all").unwrap();
    let removed = db.clean().unwrap();
    assert_eq!(removed, 1);

    let dirs = db.list_recent(10).unwrap();
    assert_eq!(dirs.len(), 0);
}

#[test]
fn test_stats() {
    let db = test_db();
    db.add("/a").unwrap();
    db.add("/b").unwrap();
    db.add("/a").unwrap();

    let stats = db.stats().unwrap();
    assert_eq!(stats.total_directories, 2);
    assert_eq!(stats.total_visits, 3);
}

#[test]
fn test_back_returns_nth_previous() {
    let db = test_db();
    db.add("/first").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.add("/second").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.add("/third").unwrap();

    // back(1) = second most recent
    let path = db.back(1).unwrap();
    assert_eq!(path, Some("/second".to_string()));

    let path = db.back(2).unwrap();
    assert_eq!(path, Some("/first".to_string()));

    let path = db.back(10).unwrap();
    assert_eq!(path, None);
}
