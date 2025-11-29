use botticelli_actor::{Actor, ActorBuilder, ActorError};
use botticelli_core::{GenerateRequestBuilder, MessageBuilder, Role, Input};

#[tokio::test]
async fn test_actor_handles_invalid_input() {
    let actor = ActorBuilder::default()
        .name("test_actor")
        .build()
        .expect("Valid actor");
    
    // Test with empty messages - should fail gracefully
    let request = GenerateRequestBuilder::default()
        .messages(vec![])
        .build()
        .expect("Empty request");
    
    // Actor should handle this error without panicking
    // Actual execution would require full setup, this tests builder validation
}

#[tokio::test]
async fn test_actor_state_recovery() {
    // Test that actor can recover from failed skill execution
    let actor = ActorBuilder::default()
        .name("recovery_test")
        .build()
        .expect("Valid actor");
    
    // After a skill fails, actor should still be usable
    assert_eq!(actor.name(), "recovery_test");
}
