#version 450

layout(push_constant) uniform PushConstants {
    mat4 view;
    vec3 camera_pos;
    vec2 resolution;
} push;

layout(location = 0) out vec4 f_color;

float terrain_max_height = 3;

vec3 hash3( in vec3 p )      // this hash is not production ready, please
{                        // replace this by something better
     p = vec3( dot(p,vec3(127.1,311.7, 74.7)),
               dot(p,vec3(269.5,183.3,246.1)),
               dot(p,vec3(113.5,271.9,124.6)));

     return -1.0 + 2.0*fract(sin(p)*43758.5453123);
}

vec2 hash2(in vec2 p) {
    return hash3(vec3(p, 0)).xy;
}

vec2 hash(in vec2 p){
    return hash2(p);
}

float hash1( vec2 p )
{
    p  = 50.0*fract( p*0.3183099 );
    return fract( p.x*p.y*(p.x+p.y) );
}


// https://iquilezles.org/articles/gradientnoise/
// returns 3D value noise (in .x)  and its derivatives (in .yz)
vec3 noised( in vec2 x )
{
    vec2 i = floor( x );
    vec2 f = fract( x );

    vec2 u = f*f*f*(f*(f*6.0-15.0)+10.0);
    vec2 du = 30.0*f*f*(f*(f-2.0)+1.0);

    vec2 ga = hash( i + vec2(0.0,0.0) );
    vec2 gb = hash( i + vec2(1.0,0.0) );
    vec2 gc = hash( i + vec2(0.0,1.0) );
    vec2 gd = hash( i + vec2(1.0,1.0) );

    float va = dot( ga, f - vec2(0.0,0.0) );
    float vb = dot( gb, f - vec2(1.0,0.0) );
    float vc = dot( gc, f - vec2(0.0,1.0) );
    float vd = dot( gd, f - vec2(1.0,1.0) );

    return vec3( va + u.x*(vb-va) + u.y*(vc-va) + u.x*u.y*(va-vb-vc+vd),   // value
                 ga + u.x*(gb-ga) + u.y*(gc-ga) + u.x*u.y*(ga-gb-gc+gd) +  // derivatives
                 du * (u.yx*(va-vb-vc+vd) + vec2(vb,vc) - va));
}

// https://www.shadertoy.com/view/4ttSWf
float noise( in vec2 x )
{
    vec2 p = floor(x);
    vec2 w = fract(x);
    #if 1
    vec2 u = w*w*w*(w*(w*6.0-15.0)+10.0);
    #else
    vec2 u = w*w*(3.0-2.0*w);
    #endif

    float a = hash1(p+vec2(0,0));
    float b = hash1(p+vec2(1,0));
    float c = hash1(p+vec2(0,1));
    float d = hash1(p+vec2(1,1));

    return -1.0+2.0*(a + (b-a)*u.x + (c-a)*u.y + (a - b - c + d)*u.x*u.y);
}

float rand(vec2 co){
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

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

vec4 opU( vec4 d1, vec4 d2 )
{
    return (d1.x<d2.x) ? d1 : d2;
}

vec4 sdDong(vec3 p) {
    float d = 1000;

    d = smin(d, sphere_sdf(p, vec3(-0.3, 0.5, 0), 0.5), 0.1);
    d = smin(d, sphere_sdf(p, vec3(0.3, 0.5, 0), 0.5), 0.1);
    d = smin(d, sdVerticalCapsule(p - vec3(0, 1, 0), 2, 0.3), 0.1);

    return vec4(d, vec3(0.9, 0.3,0.35));
}

vec4 sdFloor(vec3 p) {
    float d = 1000;
    d = smin(d, p.y - noise(p.xz*0.1) * terrain_max_height, 0.1);
    return vec4(d, vec3(0.4, 0.0, 0.9));
}

vec4 map( in vec3 p )
{
    vec4 result = vec4(1000);

    result = opU(result, sdFloor(p));
    result = opU(result, sdDong(p));

    return result;
}

float f( in vec3 p )
{
    return map(p).x;
}

//vec3 calcNormal( in vec3 p ) // for function f(p)
//{
//    const float eps = 0.00001; // or some other value
//    const vec2 h = vec2(eps,0);
//    return normalize( vec3(f(p+h.xyy) - f(p-h.xyy),
//                           f(p+h.yxy) - f(p-h.yxy),
//                           f(p+h.yyx) - f(p-h.yyx) ) );
//}

vec3 calcNormal( in vec3 p ) // for function f(p)
{
    const float h = 0.0001; // replace by an appropriate value
    const vec2 k = vec2(1,-1);
    return normalize( k.xyy*f( p + k.xyy*h ) +
    k.yyx*f( p + k.yyx*h ) +
    k.yxy*f( p + k.yxy*h ) +
    k.xxx*f( p + k.xxx*h ) );
}

float getCheckerboard(vec2 p) {
    vec2 pattern = 1+sin(p);
    return mod(int(pattern.x) + int(pattern.y), 2);
}

float calculateShadow(vec3 point, vec3 light) {
    float t = 0.02;
    float result = 1.0;
    for (int i = 0; i < 25, t < 10; i++) {
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
    float step = 0.03;
    float ao = 0;
    for (int i = 0; i < 5; i++) {
        ao += 1/exp2(i) * float(i) * step - f(point + normal * float(i) * (start + step));
    }
    return 1 - 5*ao;
}

bool march(in vec3 ray, vec3 start, out vec3 hit, out vec3 color) {
//    // floor plane
//    float t = (height - start.y) / ray.y;
//    if (t > height) {
//        hit = start + ray * t;
//        count = 1;
//        color = vec3(0.2, 0.0, 1.0) * clamp(getCheckerboard(hit.xz), 0.5, 1.0);
//        result = true;
//    }

    // terrain
//    float step = 0.01;
//    for (float terrain_distance = 0; terrain_distance < 1000; terrain_distance += step) {
//        step = 0.01 + terrain_distance * 0.01;
//        vec3 p = start + ray * terrain_distance;
//        float n = noise(p.xz*0.1);
//        if (p.y < n*10) {
//            hit = p - ray * terrain_distance * 0.5;
//            float color_height_factor = clamp((n + 1) / 2, 0.0, 1.0);
//            color = vec3(0.9, 0.9, 0.9) * color_height_factor + vec3(0.25, 0.7, 0.2) * (1 - color_height_factor);
//            object_type = 1;
//            result = true;
//        }
//    }

    // objects
    float d = 0, dist = 0;
    for (int i = 0; i < 256, dist < 1000; i++) {
        vec3 pos = start + ray * dist;
        vec4 result = map(pos);
        d = result.x;
        if (d < 0.0001 * dist) {
            hit = pos;
            color = result.yzw;
            return true;
        }
        dist += d;
    }
    return false;
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
    vec3 ray = (push.view * normalize(vec4(vec3(uv, 1.0 / tan(radians(camera_fov) / 2.0)), 1.0))).xyz;

    vec3 hit;
    vec3 color;
    bool has_hit = march(ray, origin, hit, color);

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
            0.7 * color * diffuse * shadow * ao
            + 0.04 * vec3(1.0, 1.0, 1.0) * specular
            + 0.2 * color * ambient_light
            //+ vec3(0.5, 0.5, 0.5)*(1-fog)
        , 1.0);
    } else {
        f_color = vec4(0.5, 0.5, 0.5, 1.0);
    }
}