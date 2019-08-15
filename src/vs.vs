attribute vec3 pos;
attribute vec4 vcol;
attribute vec2 vuv;
attribute float vblend;

varying vec4 col;
varying vec2 uv;
varying float blend;

uniform float aspect_ratio;

void main() {
	col = vcol;
	uv = vuv;
	blend = vblend;
	gl_Position = vec4((pos.xy)*vec2(aspect_ratio,1.0),pos.z*0.0001,1.0);
}
