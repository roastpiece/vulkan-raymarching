#version 450

layout(location = 0) out vec4 f_color;

float circle_sdf(vec2 p, float r) {
    return length(gl_FragCoord.xy - p) - r;
}

void main() {
    float circle1 = circle_sdf(vec2(150, 150), 30.0);
    float circle2 = circle_sdf(vec2(350, 255), 100.0);
    float circle3 = circle_sdf(vec2(244, 400), 70.0);

    float d = min(circle1, min(circle2, circle3));

    if (d < 0.0) {
        f_color = vec4(0.0, 1.0, 0.0, 1.0);
    } else {
        f_color = vec4(vec3(1.0, 0.0, 0.0) * sin(d), 1.0);
    }
}