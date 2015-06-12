
type Distance = f32;

struct Node<Item> {
    near: Option<Box<Node<Item>>>,
    far: Option<Box<Node<Item>>>,
    vantage_point: Item, // Pointer to the item (value) represented by the current node
    radius: Distance,    // How far the `near` node stretches
    idx: usize,             // Index of the `vantage_point` in the original items array
}

struct Handle<Item> {
    root: Box<Node<Item>>,
}

/* Temporary object used to reorder/track distance between items without modifying the orignial items array
   (also used during search to hold the two properties).
*/
struct Tmp {
    distance: Distance,
    idx: usize,
}

static int vp_compare_distance(const void *ap, const void *bp) {
    vp_distance a = ((const vp_tmp*)ap)->distance;
    vp_distance b = ((const vp_tmp*)bp)->distance;
    return a > b ? 1 : -1;
}

static void vp_sort_indexes_by_distance(const vp_item *vantage_point, vp_tmp *indexes, int num_indexes, vp_item *const items[], vp_distance_callback *get_distance) {
    for(int i=0; i < num_indexes; i++) {
        indexes[i].distance = get_distance(vantage_point, items[indexes[i].idx]);
    }
    qsort(indexes, num_indexes, sizeof(indexes[0]), vp_compare_distance);
}

static vp_node *vp_create_node(vp_tmp *indexes, int num_indexes, vp_item *const items[], vp_distance_callback *get_distance) {
    if (num_indexes <= 0) {
        return NULL;
    }

    vp_node *node = calloc(1, sizeof(node[0]));

    if (num_indexes == 1) {
        *node = (vp_node){
            .vantage_point = items[indexes[0].idx],
            .idx = indexes[0].idx,
            .radius = FLT_MAX,
        };
        return node;
    }

    const int ref_idx = indexes[0].idx;

    // Removes the `ref_idx` item from remaining items, because it's included in the current node
    indexes = &indexes[1];
    num_indexes -= 1;

    vp_sort_indexes_by_distance(items[ref_idx], indexes, num_indexes, items, get_distance);

    // Remaining items are split by the median distance
    const int half_idx = num_indexes/2;

    *node = (vp_node){
        .vantage_point = items[ref_idx],
        .idx = ref_idx,
        .radius = indexes[half_idx].distance,
    };
    node->near = vp_create_node(indexes, half_idx, items, get_distance);
    node->far = vp_create_node(&indexes[half_idx], num_indexes - half_idx, items, get_distance);

    return node;
}

/**
 * Create a Vantage Point tree for fast nearest neighbor search.
 *
 * Note that the callback must return distances that meet triangle inequality.
 * Specifically, it can't return squared distance (you must use sqrt if you use Euclidean distance)
 *
 * @param  items        Array of pointers to items that will be searched. Must not be freed until the tree is freed!
 * @param  num_items    Number of items in the array. Must be > 0
 * @param  get_distance A callback function that will calculdate distance between two items given their pointers.
 * @return              NULL on error or a handle that must be freed with vp_free().
 */
vp_handle *vp_init(vp_item *const items[], const int num_items, vp_distance_callback *const get_distance) {
    if (!items || num_items <= 0 || !get_distance) {
        return NULL;
    }

    vp_tmp indexes[num_items];

    for(int i=0; i < num_items; i++) {
        indexes[i].idx = i;
    }

    vp_handle *handle = malloc(sizeof(handle[0]));
    *handle = (vp_handle){
        .root = vp_create_node(indexes, num_items, items, get_distance),
        .get_distance = get_distance,
    };
    return handle;
}

static void vp_search_node(const vp_node *node, const vp_item *needle, vp_tmp *best_candidate, vp_distance_callback *get_distance) {
    const vp_distance distance = get_distance(node->vantage_point, needle);

    if (distance < best_candidate->distance) {
        best_candidate->distance = distance;
        best_candidate->idx = node->idx;
    }

    // Recurse towards most likely candidate first to narrow best candidate's distance as soon as possible
    if (distance < node->radius) {
        if (node->near) {
            vp_search_node(node->near, needle, best_candidate, get_distance);
        }
        // The best node (final answer) may be just ouside the radius, but not farther than
        // the best distance we know so far. The vp_search_node above should have narrowed
        // best_candidate->distance, so this path is rarely taken.
        if (node->far && distance >= node->radius - best_candidate->distance) {
            vp_search_node(node->far, needle, best_candidate, get_distance);
        }
    } else {
        if (node->far) {
            vp_search_node(node->far, needle, best_candidate, get_distance);
        }
        if (node->near && distance <= node->radius + best_candidate->distance) {
            vp_search_node(node->near, needle, best_candidate, get_distance);
        }
    }
}

/**
 * Finds item closest to given needle (that can be any item) and returns *index* of the item in items array from vp_init.
 *
 * @param  handle       VP tree from vp_init(). Must not be NULL.
 * @param  needle       The query.
 * @return              Index of the nearest item found.
 */
int vp_find_nearest(const vp_handle *handle, const vp_item *needle) {
    vp_tmp best_candidate = {
        .distance = FLT_MAX,
    };
    vp_search_node(handle->root, needle, &best_candidate, handle->get_distance);
    return best_candidate.idx;
}
