use crate::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlProgram,HtmlImageElement,WebGlShader,HtmlCanvasElement,WebSocket,MessageEvent,Blob,FileReader,Document};
use js_sys::Uint8Array;
use crate::vertex::*;
use std::mem;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;

pub fn request_animation_frame(f: &Closure<dyn FnMut()>) {
	window()
		.request_animation_frame(f.as_ref().unchecked_ref())
		.expect("should register `requestAnimationFrame` OK");
}


pub fn window() -> web_sys::Window {
	web_sys::window().expect("no global `window` exists")
}

pub fn compile_shader(
	context: &GL,
	shader_type: u32,
	source: &str,
) -> Result<WebGlShader, String> {
	let shader = context
		.create_shader(shader_type)
		.ok_or_else(|| String::from("Unable to create shader object"))?;
	context.shader_source(&shader, source);
	context.compile_shader(&shader);
	
	if context.get_shader_parameter(&shader, GL::COMPILE_STATUS).as_bool().unwrap_or(false)	{
		Ok(shader)
	} else {
		Err(context.get_shader_info_log(&shader).unwrap_or_else(|| String::from("Unknown error creating shader")))
	}
}

pub fn link_program(
	context: &GL,
	vert_shader: &WebGlShader,
	frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
	let program = context
		.create_program()
		.ok_or_else(|| String::from("Unable to create shader object"))?;
	
	context.attach_shader(&program, vert_shader);
	context.attach_shader(&program, frag_shader);
	context.link_program(&program);
	
	if context
		.get_program_parameter(&program, GL::LINK_STATUS)
		.as_bool()
		.unwrap_or(false)
	{
		Ok(program)
	} else {
		Err(context
			.get_program_info_log(&program)
			.unwrap_or_else(|| String::from("Unknown error creating program object")))
	}
}

pub fn screen_coords(x: i32, y: i32, canvas: &HtmlCanvasElement) -> Vec2<f32> {
	let x = x as f32;
	let y = -y as f32;
	let (x, y) = (x / canvas.client_width() as f32 * 2.0 - 1.0, y / canvas.client_height() as f32 * 2.0 + 1.0);
	let x = x * canvas.client_width() as f32 / canvas.client_height() as f32; //multiply by aspect ratio so it will line up with aspect ratio rendered
	vec2(x,y)
}

pub fn send(ws: &WebSocket, t: ClientPacket) -> Result<(), JsValue> {
	send_any(ws, &t)
}

pub fn send_any<T: Serialize>(ws: &WebSocket, t: &T) -> Result<(), JsValue> {
	let data = serialize(t).expect(l!());
	unsafe {
		let data = js_sys::Uint8Array::view(&data);
		ws.send_with_array_buffer_view(&data)
	}
}

pub fn recv<F: 'static + FnOnce(ServerPacket)>(e: &MessageEvent, f: F) {
	let b = Blob::from(e.data());
	let r = FileReader::new().expect("failed to create a file reader");
	let r2 = r.clone();
	let onload = Closure::once_into_js(Box::new(move|| {
		let data = Uint8Array::new(&r2.result().unwrap());
		let mut readable = vec!(0; data.length() as usize);
		data.copy_to(&mut readable);
		let p = deserialize(&readable).unwrap();
		f(p);
	}) as Box<dyn FnOnce()>);
	r.add_event_listener_with_callback("loadend", onload.unchecked_ref()).unwrap();
	r.read_as_array_buffer(&b).unwrap();
}

pub fn render(mut verts: Vec<crate::vertex::Vertex>, context: &GL) {
	let len = verts.len();
	unsafe {
		let ptr = verts.as_mut_ptr() as *mut f32; //note that transmuting like this is safe because both Vertex and Vec2/Vec3 are repr(C) so they are just a number of floats anyway
		let len = len * (STRIDE as usize) / 4;
		let cap = verts.capacity() * (STRIDE as usize);
		std::mem::forget(verts);
		let verts = Vec::from_raw_parts(ptr, len, cap);
		let verts = js_sys::Float32Array::view(&verts);
		
		context.buffer_data_with_array_buffer_view(
			GL::ARRAY_BUFFER,
			&verts,
			GL::STATIC_DRAW,
		);
	}
	
	context.draw_arrays(GL::TRIANGLES, 0, len as i32);
}

pub fn load_textures(context: Rc<GL>) {
	let image = Rc::new(RefCell::new(HtmlImageElement::new().unwrap()));
	let image2 = image.clone();
	let onload = Closure::wrap(Box::new(move|| {
		let texture = context.create_texture().expect("failed to create texture");
		context.active_texture(GL::TEXTURE0);
		context.bind_texture(GL::TEXTURE_2D, Some(&texture));
		context.pixel_storei(GL::UNPACK_FLIP_Y_WEBGL, 1);
		context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
		context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
		context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
		context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
		//context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_LOD, 0);
		//context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAX_LOD, 0);
		//context.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAX_LEVEL, 0);
		
		context.tex_image_2d_with_u32_and_u32_and_image(
			GL::TEXTURE_2D,
			0,
			GL::RGBA as i32,
			GL::RGBA,
			GL::UNSIGNED_BYTE,
			&image2.borrow(),
		).expect("");
	}) as Box<dyn Fn()>);
	image.borrow().set_onload(Some(onload.as_ref().unchecked_ref()));
	onload.forget();
	image.borrow().set_src("textures.png");
}

pub fn setup_rendering(canvas: &HtmlCanvasElement, document: &Document) -> Result<Rc<GL>, JsValue> {
	let context = Rc::new(canvas.get_context("webgl")?.expect("browser does not support webgl").dyn_into::<GL>()?);
	
	let vert_shader = compile_shader(
		&context,
		GL::VERTEX_SHADER,
		include_str!("vs.vs"),
	)?;
	let frag_shader = compile_shader(
		&context,
		GL::FRAGMENT_SHADER,
		include_str!("fs.fs"),
	)?;
	let program = link_program(&context, &vert_shader, &frag_shader)?;
	context.use_program(Some(&program));
	
	let buffer = context.create_buffer().ok_or("failed to create buffer")?;
	context.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
	
	context.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, STRIDE, 0);
	context.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, STRIDE, 12);
	context.vertex_attrib_pointer_with_i32(2, 2, GL::FLOAT, false, STRIDE, 12+16);
	context.vertex_attrib_pointer_with_i32(3, 1, GL::FLOAT, false, STRIDE, 12+16+8);
	context.enable_vertex_attrib_array(0); context.enable_vertex_attrib_array(1); context.enable_vertex_attrib_array(2); context.enable_vertex_attrib_array(3);
	
	let aspect_ratio_location = context.get_uniform_location(&program, "aspect_ratio");
	let context2 = context.clone();
	let canvas2 = canvas.clone();
	let body = document.body().expect("website had no body");
	let aspect_ratio_location2 = aspect_ratio_location.clone();
	let onresize = Closure::wrap(Box::new(move|| {
		let (w, h) = (body.client_width(), body.client_height());
		context2.viewport(0, 0, w, h);
		canvas2.set_attribute("width",&w.to_string()).expect("failed to set canvas width");
		canvas2.set_attribute("height",&h.to_string()).expect("failed to set canvas height");
		let a = w as f32 / h as f32;
		ASPECT_RATIO.store(unsafe { mem::transmute(a) }, Relaxed);
		context2.uniform1f(aspect_ratio_location2.as_ref(), a.recip());
	}) as Box<dyn Fn()>);
	window().add_event_listener_with_callback("resize",onresize.as_ref().unchecked_ref()).expect("failed to add resize listener");
	onresize.forget();
	
	load_textures(context.clone());
	
	let body = document.body().expect("no body present on document");
	let (w, h) = (body.client_width(), body.client_height());
	canvas.set_attribute("width",&w.to_string()).expect("failed to set canvas width");
	canvas.set_attribute("height",&h.to_string()).expect("failed to set canvas height");
	context.viewport(0, 0, w, h);
	let a = w as f32 / h as f32;
	ASPECT_RATIO.store(unsafe { mem::transmute(a) }, Relaxed);
	context.uniform1f(aspect_ratio_location.as_ref(), a.recip());
	
	context.clear_color(0.0, 0.0, 0.0, 1.0);
	context.enable(GL::DEPTH_TEST);
	context.depth_func(GL::GEQUAL);
	context.enable(GL::BLEND);
	context.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
	
	Ok(context)
}

static ASPECT_RATIO: AtomicU32 = AtomicU32::new(unsafe { mem::transmute(1f32) });

pub fn top_edge() -> f32 {
	1.0
}

pub fn bottom_edge() -> f32 {
	-1.0
}

pub fn left_edge() -> f32 {
	- unsafe { mem::transmute/*::<_, f32>*/(ASPECT_RATIO.load(Relaxed)) }
}

pub fn right_edge() -> f32 {
	unsafe { mem::transmute(ASPECT_RATIO.load(Relaxed)) }
}

pub fn hash(thing: &[u8]) -> [u64; 4] {
	use sha3::{Sha3_256,Digest};
	let mut hasher = Sha3_256::default();
	hasher.input(thing);
	unsafe { mem::transmute(hasher.result()) }
}

pub fn splittify(s: &mut Vec<char>, max_width: usize) {
	let mut i = 0;
	let mut last_nl = 0;
	let mut last_space = max_width;
	while i < s.len() {
		match s[i] {
			'\n' => { last_nl = i; last_space = i + max_width; },
			' ' => last_space = i,
			_ => {},
		}
		if last_nl + max_width <= i {
			s[last_space] = '\n';
			last_nl = last_space;
			last_space = i + max_width;
		}
		i += 1;
	}
}
