use super::*;

#[test]
fn test_io_parse_basic() {
    let toml = r#"
        [[input]]
        name = "retina"
        entry_z = "top"
        matrix_id_v1 = { id = "m1" }

        [[input.pin]]
        name = "retina_edges"
        pin_id_v1 = { id = "p1" }
        width = 64
        height = 64
		stride = 1
        local_u = 0.0
        local_v = 0.0
        u_width = 1.0
        v_height = 1.0
        target_type = "L4_Stellate"
        
        [[input]]
        name = "audio"
        entry_z = "mid"
        matrix_id_v1 = { id = "m2" }

        [[input.pin]]
        name = "audio_spectrogram"
        pin_id_v1 = { id = "p2" }
        width = 128
        height = 16
		stride = 1
        local_u = 0.0
        local_v = 0.0
        u_width = 1.0
        v_height = 1.0
        target_type = "ALL"
    "#;

    let io = IoConfig::parse(toml).unwrap();
    assert_eq!(io.input.len(), 2);
    
    assert_eq!(io.input[0].name, "retina");
    assert_eq!(io.input[0].pin[0].name, "retina_edges");
    assert_eq!(io.input[0].pin[0].width, 64);
    assert_eq!(io.input[0].pin[0].height, 64);
    
    assert_eq!(io.input[1].name, "audio");
    assert_eq!(io.input[1].pin[0].name, "audio_spectrogram");
    assert_eq!(io.input[1].pin[0].width, 128);
    assert_eq!(io.input[1].pin[0].height, 16);
}

#[test]
fn test_io_parse_with_outputs() {
    let toml = r#"
        [[output]]
        name = "motor"
        entry_z = "bottom"
        matrix_id_v1 = { id = "m3" }

        [[output.pin]]
        name = "motor_arm"
        pin_id_v1 = { id = "p3" }
        width = 8
        height = 8
		stride = 1
        local_u = 0.0
        local_v = 0.0
        u_width = 1.0
        v_height = 1.0
        target_type = "L5_Pyramidal"
        
        [[output]]
        name = "motor_leg_matrix"
        entry_z = "bottom"
        matrix_id_v1 = { id = "m4" }

        [[output.pin]]
        name = "motor_leg"
        pin_id_v1 = { id = "p4" }
        width = 4
        height = 4
		stride = 1
        local_u = 0.0
        local_v = 0.0
        u_width = 1.0
        v_height = 1.0
        target_type = "ALL"
    "#;

    let io = IoConfig::parse(toml).unwrap();
    assert_eq!(io.output.len(), 2);
    
    assert_eq!(io.output[0].pin[0].name, "motor_arm");
    assert_eq!(io.output[0].pin[0].width, 8);
    
    assert_eq!(io.output[1].pin[0].name, "motor_leg");
    assert_eq!(io.output[1].pin[0].width, 4);
}
