#version 450

layout(push_constant) uniform PushConstants {
    vec2 resolution;
} push;

layout(location = 0) out vec4 f_color;

float sphere_sdf(vec3 observer, vec3 target, float r) {
    return length(observer - target) - r;
}

float floor_sdf(vec3 observer, float y) {
    return y - observer.y;
}

float sdVerticalCapsule( vec3 p, float h, float r )
{
    p.y -= clamp( p.y, 0.0, h );
    return length( p ) - r;
}

float smin( float a, float b, float k )
{
    float h = max( k-abs(a-b), 0.0 )/k;
    return min( a, b ) - h*h*k*(1.0/4.0);
}

float f( in vec3 p )
{
    float sphere = sphere_sdf(p, vec3(push.resolution.x/2+150, push.resolution.y/2, 2000), 200.0);
    float sphere2 = sphere_sdf(p, vec3(push.resolution.x/2-150, push.resolution.y/2, 2000), 200.0);
    float capsule = sdVerticalCapsule(p - vec3(push.resolution.x/2, push.resolution.y/2 - 1000, 2000), 800, 100);
    float floor = floor_sdf(p, push.resolution.y);
    return min(floor, smin(smin(sphere2, sphere, 100), capsule, 50));
}

vec3 calcNormal( in vec3 p ) // for function f(p)
{
    const float eps = 0.001; // or some other value
    const vec2 h = vec2(eps,0);
    return normalize( vec3(f(p+h.xyy) - f(p-h.xyy),
                           f(p+h.yxy) - f(p-h.yxy),
                           f(p+h.yyx) - f(p-h.yyx) ) );
}


bool march(in vec3 ray, vec3 start, out vec3 hit) {
    float d = 0;
    vec3 normalized = normalize(ray);
    for (int i = 0; i < 10000; i++) {
        start = start + normalized * d;
        d = f(start);
        if (d < 0.01) {
            hit = start;
            return true;
        }
    }
    return false;
}

void main() {
    vec3 origin = vec3(push.resolution.x/2, push.resolution.y/2, -1000.0);

    vec3 ray = vec3(gl_FragCoord.xy, 0.0) - origin;

    vec3 hit;
    bool has_hit = march(ray, vec3(gl_FragCoord.xy, 0.0), hit);
    vec3 light = normalize(vec3(-1.0, -1.0, -0.5));

    if (has_hit) {
        vec3 normal = calcNormal(hit);
        float intensity = dot(normal, light);
        float distance = length(hit - origin);
        float fog = min(1, 100000000.0 / (distance * distance));

        f_color = vec4(vec3(0.0, 1.0, 0.0) * intensity * fog + vec3(0.5, 0.5, 0.5)*(1-fog), 1.0);
    } else {
        f_color = vec4(0.5, 0.5, 0.5, 1.0);
    }
}