use midir::{MidiOutput, MidiOutputPort};
use std::{time::Duration, io::{stdin, stdout, Write}};
use std::thread::sleep;

fn main() {
    match run() {
        Ok(()) => (),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the MIDI output
    let midi_out = MidiOutput::new("MIDI Output").expect("Failed to create MIDI output");
    let out_ports: Vec<MidiOutputPort> = midi_out.ports();

    let out_port: &MidiOutputPort = match out_ports.len() {
        0 => {
            println!("No MIDI output ports available");
            return Err("No MIDI output ports available".into());
        },
        1 => {
            println!("Using the only available MIDI output port");
            &out_ports[0]
        },
        _ => {
            print!("Please select output port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            out_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid output port selected")?
        }
    };

    let mut conn_out = midi_out.connect(out_port, "midir-test")?;
    println!("Connection open. Listen!");
    {
        // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
        let mut play_note = |note: u8, duration: u64| {
            const NOTE_ON_MSG: u8 = 0x90;
            const NOTE_OFF_MSG: u8 = 0x80;
            const VELOCITY: u8 = 0x64;
            // We're ignoring errors in here
            let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
            sleep(Duration::from_millis(duration * 15));
            let _ = conn_out.send(&[NOTE_OFF_MSG, note, VELOCITY]);
        };

        sleep(Duration::from_millis(4 * 150));

        play_note(66, 4);
        play_note(65, 3);
        play_note(63, 1);
        play_note(61, 6);
        play_note(59, 2);
        play_note(58, 4);
        play_note(56, 4);
        play_note(54, 4);
    }
    sleep(Duration::from_millis(150));
    Ok(())
}

