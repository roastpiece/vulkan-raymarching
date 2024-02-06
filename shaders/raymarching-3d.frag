#version 450

layout(push_constant) uniform PushConstants {
    vec2 resolution;
} push;

layout(location = 0) out vec4 f_color;

float sphere_sdf(vec3 observer, vec3 target, float r) {
    return length(observer - target) - r;
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

vec2 opU( vec2 d1, vec2 d2 )
{
    return (d1.x<d2.x) ? d1 : d2;
}

float f( in vec3 p )
{
    float d = 1000;
    d = smin(d, sphere_sdf(p, vec3(-0.3, 0.5, 0), 0.5), 0.1);
    d = smin(d, sphere_sdf(p, vec3(0.3, 0.5, 0), 0.5), 0.1);
    d = smin(d, sdVerticalCapsule(p - vec3(0, 1, 0), 2, 0.3), 0.1);
    return d;
}

vec3 calcNormal( in vec3 p ) // for function f(p)
{
    if (p.y < 0.01) return vec3(0, 1, 0);

    const float eps = 0.0001; // or some other value
    const vec2 h = vec2(eps,0);
    return normalize( vec3(f(p+h.xyy) - f(p-h.xyy),
                           f(p+h.yxy) - f(p-h.yxy),
                           f(p+h.yyx) - f(p-h.yyx) ) );
}


bool march(in vec3 ray, vec3 start, out vec3 hit, out int count) {
    bool result = false;

    // floor plane
    float t = (0 - start.y) / ray.y;
    if (t > 0) {
        hit = start + ray * t;
        count = 1;
        result = true;
    }

    float d = 0, dist = 0;
    for (int i = 0; i < 100, dist < 1000; i++) {
        start = start + ray * d;
        dist += d;
        d = f(start);
        if (d < 0.01) {
            hit = start;
            count = i;
            return true;
        }
    }
    count = -1;
    return result;
}

void main() {
    vec3 camera_pos = vec3(0, 1.6, -5.0);
    vec3 camera_dir = vec3(0, 0, 1.0);
    float camera_fov = 90;
    vec2 uv = (gl_FragCoord.xy / push.resolution.xy) * 2.0 - 1.0;
    uv.y = -uv.y;

    vec3 origin = camera_pos;
    vec3 ray = normalize(vec3(uv, 1.0 / tan(radians(camera_fov) / 2.0)));

    vec3 hit;
    int count;
    bool has_hit = march(ray, origin, hit, count);
    vec3 light = normalize(vec3(-1.0, 1.0, -1));

    if (has_hit) {
        vec3 normal = calcNormal(hit);
        float diffuse = clamp(dot(normal, light), 0.0, 1.0);
        float specular = pow(clamp(dot(normal, light-ray), 0.0, 1.0), 16);

        float distance = length(hit - origin);
        float fog = min(1, 1000.0 / (distance * distance));

        f_color = vec4(
            0.6 *vec3(0.0, 1.0, 0.0) * diffuse * fog
            //+ 0.5 * vec3(1.0, 1.0, 1.0) * specular
            + vec3(0.5, 0.5, 0.5)*(1-fog), 1.0
        );
    } else {
        f_color = vec4(0.5, 0.5, 0.5, 1.0);
    }
}