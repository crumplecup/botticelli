// @generated automatically by Diesel CLI.

diesel::table! {
    act_executions (id) {
        id -> Int4,
        execution_id -> Int4,
        act_name -> Text,
        sequence_number -> Int4,
        model -> Nullable<Text>,
        temperature -> Nullable<Float4>,
        max_tokens -> Nullable<Int4>,
        response -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    act_inputs (id) {
        id -> Int4,
        act_execution_id -> Int4,
        input_order -> Int4,
        input_type -> Text,
        text_content -> Nullable<Text>,
        mime_type -> Nullable<Text>,
        source_type -> Nullable<Text>,
        source_url -> Nullable<Text>,
        source_base64 -> Nullable<Text>,
        source_binary -> Nullable<Bytea>,
        source_size_bytes -> Nullable<Int8>,
        content_hash -> Nullable<Text>,
        filename -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    model_responses (id) {
        id -> Uuid,
        created_at -> Timestamp,
        #[max_length = 50]
        provider -> Varchar,
        #[max_length = 100]
        model_name -> Varchar,
        request_messages -> Jsonb,
        request_temperature -> Nullable<Float4>,
        request_max_tokens -> Nullable<Int4>,
        #[max_length = 100]
        request_model -> Nullable<Varchar>,
        response_outputs -> Jsonb,
        duration_ms -> Nullable<Int4>,
        error_message -> Nullable<Text>,
    }
}

diesel::table! {
    narrative_executions (id) {
        id -> Int4,
        narrative_name -> Text,
        narrative_description -> Nullable<Text>,
        started_at -> Timestamp,
        completed_at -> Nullable<Timestamp>,
        status -> Text,
        error_message -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::joinable!(act_executions -> narrative_executions (execution_id));
diesel::joinable!(act_inputs -> act_executions (act_execution_id));

diesel::allow_tables_to_appear_in_same_query!(
    act_executions,
    act_inputs,
    model_responses,
    narrative_executions,
);
