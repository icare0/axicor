use super::*;

#[test]
fn test_io_parse_basic() {
    let toml = r#"
        [[input]]
        name = "retina_edges"
        target_zone = "V1"
        target_type = "L4_Stellate"
        width = 64
        height = 64
        
        [[input]]
        name = "audio_spectrogram"
        target_zone = "A1"
        target_type = "ALL"
        width = 128
        height = 16
    "#;

    let io = IoConfig::parse(toml).unwrap();
    assert_eq!(io.inputs.len(), 2);
    
    assert_eq!(io.inputs[0].name, "retina_edges");
    assert_eq!(io.inputs[0].target_zone, "V1");
    assert_eq!(io.inputs[0].target_type, "L4_Stellate");
    assert_eq!(io.inputs[0].width, 64);
    assert_eq!(io.inputs[0].height, 64);
    
    assert_eq!(io.inputs[1].name, "audio_spectrogram");
    assert_eq!(io.inputs[1].target_zone, "A1");
    assert_eq!(io.inputs[1].target_type, "ALL");
    assert_eq!(io.inputs[1].width, 128);
    assert_eq!(io.inputs[1].height, 16);
}
