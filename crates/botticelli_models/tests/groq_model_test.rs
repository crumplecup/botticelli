use botticelli_models::GroqModel;
use strum::IntoEnumIterator;

#[test]
fn test_move_up_from_middle() {
    assert_eq!(
        GroqModel::Mixtral8x7B.move_up(),
        Some(GroqModel::Llama31_70BVersatile)
    );
}

#[test]
fn test_move_up_from_top() {
    assert_eq!(GroqModel::Llama33_70BVersatile.move_up(), None);
}

#[test]
fn test_move_down_from_middle() {
    assert_eq!(
        GroqModel::Mixtral8x7B.move_down(),
        Some(GroqModel::Llama31_8BInstant)
    );
}

#[test]
fn test_move_down_from_bottom() {
    assert_eq!(GroqModel::Llama31_8BInstant.move_down(), None);
}

#[test]
fn test_as_str_matches_display() {
    for model in GroqModel::iter() {
        assert_eq!(model.as_str(), model.to_string());
    }
}
