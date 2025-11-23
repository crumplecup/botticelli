//! Tests for knowledge table access.

use botticelli_actor::KnowledgeTable;

#[test]
fn test_knowledge_table_new() {
    let table = KnowledgeTable::new("test_table");
    assert_eq!(table.name(), "test_table");
}

#[test]
fn test_knowledge_table_name() {
    let table = KnowledgeTable::new("my_knowledge_table");
    assert_eq!(table.name(), "my_knowledge_table");
}

#[test]
fn test_knowledge_table_clone() {
    let table1 = KnowledgeTable::new("table1");
    let table2 = table1.clone();
    assert_eq!(table1.name(), table2.name());
}
