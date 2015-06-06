
typedef float vp_distance;
typedef void vp_item;
typedef vp_distance vp_distance_callback(const vp_item *a, const vp_item *b);

struct vp_handle;
typedef struct vp_handle vp_handle;

vp_handle *vp_init(vp_item *const item_pointers[], const int num_items, vp_distance_callback *const compare);

int vp_find_nearest(const vp_handle *vp, const vp_item *needle);

void vp_free(vp_handle *);
