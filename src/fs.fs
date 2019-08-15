varying highp vec4 col;
varying highp vec2 uv;
varying highp float blend;

uniform sampler2D tex;

void main() {
	highp vec4 c = (1.0 - blend) * col + blend * texture2D(tex,uv);
	gl_FragColor = vec4(c.rgb * c.a,c.a);
	//gl_FragColor = (1.0 - blend) * col + blend * texture2D(tex,uv);
}
