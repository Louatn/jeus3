//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
use std::usize;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};
use std::io::ErrorKind;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct Image{
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Motion{
    point: Point,
    id: usize,
}
fn load_image(path: &str) -> Result<Image, Box<dyn std::error::Error>>{

    let content = std::fs::read(path)?;
    let mut words = std::str::from_utf8(&content)?
    .lines()
    .map(|l| l.find('#').map_or(l, |pos| &l[0..pos]))
    .flat_map(|l| l.split_whitespace())
    .filter(|w| !w.is_empty());

    //overread "P3"
    words.next();

    let width:usize = words.next().ok_or("missing width")?.parse::<usize>()?;
    let height:usize = words.next().ok_or("missing height")?.parse::<usize>()?;

    let mut pixels:Vec<Color> = Vec::with_capacity(width*height);
    for _ in 0..(width*height){
        let r:u8 = words.next().unwrap().parse::<u8>()?;
        let g:u8 = words.next().unwrap().parse::<u8>()?;
        let b:u8 = words.next().unwrap().parse::<u8>()?;
        pixels.push(Color{r,g,b});
    }
    println!("DEBUG COULEUR FOND -> R:{} G:{} B:{}",pixels[0].r,pixels[0].g,pixels[0].b);

    Ok(Image {
        width: width,
        height: height,
        pixels,
    })
}

fn draw_image(
    screen: &mut Screen,
    image: &Image,
    position: &Point,
    transparent_color: Option<&Color>,
){

    let p0 = Point {
    x: position.x.clamp(0, screen.width as i32),
    y: position.y.clamp(0, screen.height as i32),
    };
    let p1 = Point {
    x: (position.x + image.width as i32).clamp(0, screen.width as i32),
    y: (position.y + image.height as i32).clamp(0, screen.height as i32),
    };
    if p1.x > p0.x {
        let w = (p1.x - p0.x) as usize;
        let dx = 0.max(p0.x - position.x);
        let dy = 0.max(p0.y - position.y);
        let mut i_idx = dy as usize * image.width + dx as usize;
        let mut s_idx = p0.y as usize * screen.width + p0.x as usize;
        for _ in p0.y..p1.y {
            let src = &image.pixels[i_idx..i_idx + w];
            let dst = &mut screen.pixels[s_idx..s_idx + w];

             
             match transparent_color {
                None => {
                    dst.copy_from_slice(src);
                }
                Some(tr) => {
                    for x in 0..w {
                        if !(src[x].r == tr.r && src[x].g == tr.g && src[x].b == tr.b) {
                            dst[x] = src[x];
                        }
                    }
                }
             }
            i_idx += image.width;
            s_idx += screen.width;
        }
    }
}


#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
fn game_client_init(
    argc: std::ffi::c_int,
    argv: *const *const std::ffi::c_char,
    inout_width: &mut std::ffi::c_int,
    inout_height: &mut std::ffi::c_int,
    inout_dt: &mut std::ffi::c_double,
) -> *mut std::ffi::c_void /* application */ {
    let args_utf8 = Vec::from_iter((0..argc).map(|a| {
        let c_ptr = unsafe { argv.offset(a as isize) };
        let c_str = unsafe { std::ffi::CStr::from_ptr(*c_ptr) };
        c_str.to_string_lossy()
    }));
    let args = Vec::from_iter(args_utf8.iter().map(|a| a.as_ref()));
    let mut w = *inout_width as usize;
    let mut h = *inout_height as usize;
    let mut dt = *inout_dt;
    match init_application(&args, &mut w, &mut h, &mut dt) {
        Ok(app) => {
            *inout_width = w as std::ffi::c_int;
            *inout_height = h as std::ffi::c_int;
            *inout_dt = dt as std::ffi::c_double;
            Box::into_raw(Box::new(app)) as *mut _
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
fn game_client_update(
    c_evt: *const std::ffi::c_char,
    x: std::ffi::c_int,
    y: std::ffi::c_int,
    w: std::ffi::c_int,
    h: std::ffi::c_int,
    btn: std::ffi::c_int,
    c_key: *const std::ffi::c_char,
    c_screen: *mut std::ffi::c_char,
    c_app: *mut std::ffi::c_void,
) -> std::ffi::c_int /* -1: quit    0: go-on    1: redraw */ {
    let evt = unsafe { std::ffi::CStr::from_ptr(c_evt) }.to_string_lossy();
    let key = unsafe { std::ffi::CStr::from_ptr(c_key) }.to_string_lossy();
    let point = Point { x, y };
    let mut screen = Screen {
        width: w as usize,
        height: h as usize,
        pixels: unsafe {
            std::slice::from_raw_parts_mut(
                c_screen as *mut Color,
                (w * h) as usize,
            )
        },
    };
    let app = unsafe { &mut *(c_app as *mut Application) };
    let status = update_application(
        evt.as_ref(),
        key.as_ref(),
        btn as usize,
        &point,
        &mut screen,
        app,
    )
    .unwrap_or_else(|e| {
        eprintln!("ERROR: {}", e);
        UpdateStatus::Quit
    });
    match status {
        UpdateStatus::GoOn => 0,
        UpdateStatus::Redraw => 1,
        UpdateStatus::Quit => {
            // ensure deallocation
            let _owned = unsafe { Box::from_raw(app) };
            -1
        }
    }
}

//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[derive(Debug)]
struct Screen<'a> {
    width: usize,
    height: usize,
    pixels: &'a mut [Color],
}

#[derive(Debug, Clone, Copy)]
enum UpdateStatus {
    GoOn,
    Redraw,
    Quit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Application {
    myself: Player,
    players: Vec<Player>,
    status: UpdateStatus,
    input: BufReader<TcpStream>,
    output: TcpStream,
}

#[derive(Debug, Serialize, Deserialize)]
struct Player{
    id: usize,
    position: Point,
    image: Image,
}

fn init_application(
    args: &[&str],
    width: &mut usize,
    height: &mut usize,
    dt: &mut f64,
) -> Result<Application, Box<dyn std::error::Error>> {
    println!("args: {:?}", args);
    *width = 800;
    *height = 600;
    *dt = 1.0 / 30.0;
    println!("{}×{}@{:.3}", width, height, dt);

    let server_name = args.get(3).ok_or("missing server name")?.to_string();
    let server_port = args.get(4).ok_or("missing server port")?.parse::<u16>()?;

    println!("connecting to server {}:{}", server_name, server_port);
    
    let stream = TcpStream::connect((server_name, server_port))?;
    let mut output = stream.try_clone()?;
    let  input = BufReader::new(stream);

    let image = load_image(args[2]).unwrap();
    let msg = format!("newplayer {}\n", serde_json::to_string(&image)?);
        output.write_all(msg.as_bytes())?;




    Ok(Application {
        myself: Player {
            id: 0,
            position: Point { x: 0, y: 0 },
            image,
        },
        players: Vec::new(),
        status: UpdateStatus::GoOn,
        input,
        output,
    })
}

fn update_application(
    evt: &str,
    key: &str,
    btn: usize,
    point: &Point,
    screen: &mut Screen,
    app: &mut Application,
) -> Result<UpdateStatus, Box<dyn std::error::Error>> {
    let _maybe_unused = /* prevent some warnings */ (btn, point);
    if evt != "T" {
        println!(
            "evt={:?} btn={} key={:?} ({};{}) {}×{}",
            evt, btn, key, point.x, point.y, screen.width, screen.height
        );
    }

    app.status = UpdateStatus::GoOn;

    if let Some(motion) = handle_event(app, evt, key) {
        println!("motion: {:?}", motion);
        //on envoie un objet Point au serveur
        let msg = format!("motion {}\n", serde_json::to_string(&(&motion.id, &motion.point))?);
        app.output.write_all(msg.as_bytes())?;
    }
    handle_messages(app)?;
    redraw_if_needed(app, screen);
    Ok(app.status)
}


fn handle_event(
    app: &mut Application,
    evt: &str,
    key: &str,
) -> Option<Motion> {
    let mut motion = None;
    let id = app.myself.id;
    match evt {
        "C" => app.status = UpdateStatus::Redraw,
        "Q" => app.status = UpdateStatus::Quit,
        "KP" => match key {
            "Escape" => app.status = UpdateStatus::Quit,
            "Left" => motion = Some(Motion { point: Point { x: -10, y: 0 }, id }),
            "Right" => motion = Some(Motion { point: Point { x: 10, y: 0 }, id }),
            "Up" => motion = Some(Motion { point: Point { x: 0, y: -10 }, id }),
            "Down" => motion = Some(Motion { point: Point { x: 0, y: 10 }, id }),
            " " => app.status = UpdateStatus::Redraw,
            _ => {}
        },
        _ => {}
    }
    motion
}


fn handle_messages(
    app: &mut Application,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = read_lines_nonblocking(&mut app.input)?;

    for line in msg {
        if line.is_empty() {
            println!("server closed connection");
            app.status = UpdateStatus::Quit;
        } else {
        // on sépare la commande et les données ( attention ce n'est pas la méthode du cours je crois)
            match line.split_once(" ") {
                Some(("position", data)) => {
                    //on récupère l'objet Point envoyé par le serveur
                    let (id, pos) = serde_json::from_str::<(usize, Point)>(data.trim())?;
                    println!("received position: {:?}", (id, pos));
                    if id == app.myself.id {
                        app.myself.position.x += pos.x;
                        app.myself.position.y += pos.y;
                    }
                    else {
                        let mut found = false;
                        for p in app.players.iter_mut() {
                            if p.id == id {
                                p.position.x += pos.x;
                                p.position.y += pos.y;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            println!("Warning: received position for unknown player id={}", id);
                        }
                    }
                    app.status = UpdateStatus::Redraw;
                }
                Some(("added", data)) => {
                    let (id, position, players) = serde_json::from_str::<(usize, Point, Vec<Player>)>(data.trim())?;
                    app.myself.position = position;
                    app.myself.id = id;
                    app.players = players;
                }

                Some(("newplayerconnect", data)) => {
                    let (id, position, image) = serde_json::from_str::<(usize, Point, Image)>(data.trim())?;
                    println!("New player connected: id={}, position={:?}, image size={}x{}", id, position, image.width, image.height);
                    app.players.push(Player { id, position, image });
                    app.status = UpdateStatus::Redraw;
                }

                Some(("disconnect", data)) => {
                    let disconnected_id = serde_json::from_str::<usize>(data.trim())?;
                    println!("Player disconnected: id={}", disconnected_id);
                    app.players.retain(|p| p.id != disconnected_id);
                    app.status = UpdateStatus::Redraw;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn redraw_if_needed(
    app: &Application,
    screen: &mut Screen,
) {
    if let UpdateStatus::Redraw = app.status {
        for c in screen.pixels.iter_mut() {
            *c = Color { r: 0, g: 0, b: 0 };
        }
        let transparent_color: Option<&Color> = Some(&Color { r: 0, g: 0, b: 255 });
        draw_image(screen, &app.myself.image, &app.myself.position, transparent_color);
        for player in &app.players {
            draw_image(screen, &player.image, &player.position, transparent_color);
        }
    }
}


fn read_lines_nonblocking(
    input: &mut BufReader<TcpStream>
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    fn inner(
        input: &mut BufReader<TcpStream>
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            match input.read_line(&mut line) {
                Ok(r) => {
                    if !line.is_empty() {
                        lines.push(line);
                    }
                    if r == 0 {
                        lines.push(String::new()); // EOF
                        break;
                    }
                }
                Err(e) => {
                    if e.kind() != ErrorKind::WouldBlock {
                        Err(e)?
                    }
                    if line.is_empty() {
                        // line not started, don't wait for the end
                        break;
                    }
                }
            }
        }
        Ok(lines)
    }
    input.get_mut().set_nonblocking(true)?;
    let result = inner(input);
    input.get_mut().set_nonblocking(false)?;
    result
}


//~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
