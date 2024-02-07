#version 450

layout(push_constant) uniform PushConstants {
    mat4 view;
    vec3 camera_pos;
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
    if (p.y < 0.02) return vec3(0, 1, 0);

    const float eps = 0.0001; // or some other value
    const vec2 h = vec2(eps,0);
    return normalize( vec3(f(p+h.xyy) - f(p-h.xyy),
                           f(p+h.yxy) - f(p-h.yxy),
                           f(p+h.yyx) - f(p-h.yyx) ) );
}

float getCheckerboard(vec2 p) {
    vec2 pattern = 1+sin(p);
    return mod(int(pattern.x) + int(pattern.y), 2);
}

float calculateShadow(vec3 point, vec3 light) {
    float t = 0.02;
    float result = 1.0;
    for (int i = 0; i < 100, t < 10; i++) {
        float d = f(point + light * t);
        result = min(result, 16.0 * d / t);
        if (result < 0.004) {
            break;
        }
        t += d;
    }
    return result;
}

float calculateAO(vec3 point, vec3 normal) {
    float start = 0.01;
    float step = 0.04;
    float ao = 0;
    for (int i = 0; i < 5; i++) {
        ao += 1/exp2(i) * float(i) * step - f(point + normal * float(i) * (start + step));
    }
    return 1 - 5*ao;
}

bool march(in vec3 ray, vec3 start, out vec3 hit, out int count, out vec3 color) {
    bool result = false;

    // floor plane
    float t = (0 - start.y) / ray.y;
    if (t > 0) {
        hit = start + ray * t;
        count = 1;
        color = vec3(0.2, 0.0, 1.0) * clamp(getCheckerboard(hit.xz), 0.5, 1.0);
        result = true;
    }

    float d = 0, dist = 0;
    for (int i = 0; i < 100, dist < 1000; i++) {
        start = start + ray * d;
        dist += d;
        d = f(start);
        if (d < 0.0001) {
            color = vec3(0.9, 0.3,0.35);
            hit = start;
            count = i;
            return true;
        }
    }
    count = -1;
    return result;
}

//mat3 getViewMatrix(vec3 camera_pos, vec3 camera_dir) {
//    vec3 right = normalize(cross(vec3(0, 1, 0), camera_dir));
//    vec3 up = normalize(cross(camera_dir, right));
//    return mat3(right, up, camera_dir);
//}

void main() {
    float camera_fov = 90;
    vec2 aspectRatio = vec2(push.resolution.x / push.resolution.y, 1.0);
    vec2 uv = (gl_FragCoord.xy / push.resolution.xy) * 2.0 - 1.0;
    uv *= aspectRatio;
    uv.y = -uv.y;

    vec3 origin = push.camera_pos;
    vec3 ray = (push.view * normalize(vec4(uv, 1.0 / tan(radians(camera_fov) / 2.0), 0.0))).xyz;

    vec3 hit;
    int count;
    vec3 color;
    bool has_hit = march(ray, origin, hit, count, color);

    if (has_hit) {
        vec3 normal = calcNormal(hit);
        vec3 light = normalize(vec3(-1.0, 1.0, -1));
        float diffuse = clamp(dot(normal, light), 0.0, 1.0);
        float specular = pow(clamp(dot(normal, light-ray), 0.0, 1.0), 16);
        float shadow = clamp(calculateShadow(hit, light), 0.2, 1.0);
        float ao = clamp(calculateAO(hit, normal), 0.1, 1.0);

        float distance = length(hit - origin);
        float fog = min(1, 5000.0 / (distance * distance));

        float ambient_light = 0.5;
        f_color = vec4(
            0.7 * color * fog * ao * shadow * diffuse
            + 0.02 * vec3(1.0, 1.0, 1.0) * specular
            + 0.2 * color * ambient_light
            + vec3(0.5, 0.5, 0.5)*(1-fog), 1.0
        );
    } else {
        f_color = vec4(0.5, 0.5, 0.5, 1.0);
    }
}