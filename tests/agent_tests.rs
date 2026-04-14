//! Tests for the Agent module

use holodeck_rust::agent::Agent;
use holodeck_rust::permission::Permission;

#[test]
fn test_agent_new_creates_agent_with_default_values() {
    let agent = Agent::new("TestAgent", "test-room");

    assert_eq!(agent.name, "TestAgent");
    assert_eq!(agent.room_id, "test-room");
    assert_eq!(agent.permission, Permission::Crew);
    assert_eq!(agent.energy, 1000);
    assert!(agent.connected_at > 0);
}

#[test]
fn test_agent_new_with_different_names() {
    let agent1 = Agent::new("Alice", "room-1");
    let agent2 = Agent::new("Bob", "room-2");

    assert_eq!(agent1.name, "Alice");
    assert_eq!(agent2.name, "Bob");
    assert_eq!(agent1.room_id, "room-1");
    assert_eq!(agent2.room_id, "room-2");
}

#[test]
fn test_agent_new_default_permission_is_crew() {
    let agent = Agent::new("TestAgent", "test-room");
    assert_eq!(agent.permission, Permission::Crew);
}

#[test]
fn test_agent_new_default_energy_is_1000() {
    let agent = Agent::new("TestAgent", "test-room");
    assert_eq!(agent.energy, 1000);
}

#[test]
fn test_agent_new_connected_at_is_set() {
    let agent = Agent::new("TestAgent", "test-room");
    assert!(agent.connected_at > 0);

    // Verify it's a recent timestamp (within last 10 seconds)
    let now = chrono::Utc::now().timestamp();
    assert!((now - agent.connected_at).abs() < 10);
}
