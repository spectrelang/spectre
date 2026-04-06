#include "yyjson.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

size_t sx_yyjson_get_len(void *v)
{
    return yyjson_get_len((yyjson_val *)v);
}

uint8_t sx_yyjson_get_type(void *v)
{
    return yyjson_get_type((yyjson_val *)v);
}

uint8_t sx_yyjson_get_subtype(void *v)
{
    return yyjson_get_subtype((yyjson_val *)v);
}

uint8_t sx_yyjson_get_tag(void *v)
{
    return yyjson_get_tag((yyjson_val *)v);
}

const char *sx_yyjson_get_type_desc(void *v)
{
    return yyjson_get_type_desc((yyjson_val *)v);
}

const char *sx_yyjson_get_raw(void *v)
{
    return yyjson_get_raw((yyjson_val *)v);
}

bool sx_yyjson_get_bool(void *v)
{
    return yyjson_get_bool((yyjson_val *)v);
}

void *sx_yyjson_mut_doc_new(void *alc)
{
    return (void *)yyjson_mut_doc_new((const yyjson_alc *)alc);
}

uint64_t sx_yyjson_get_uint(void *v)
{
    return yyjson_get_uint((yyjson_val *)v);
}

int64_t sx_yyjson_get_sint(void *v)
{
    return yyjson_get_sint((yyjson_val *)v);
}

int64_t sx_yyjson_get_int(void *v)
{
    return yyjson_get_int((yyjson_val *)v);
}

double sx_yyjson_get_real(void *v)
{
    return yyjson_get_real((yyjson_val *)v);
}

double sx_yyjson_get_num(void *v)
{
    return yyjson_get_num((yyjson_val *)v);
}

const char *sx_yyjson_get_str(void *v)
{
    return yyjson_get_str((yyjson_val *)v);
}

size_t sx_yyjson_arr_size(void *arr)
{
    return yyjson_arr_size((yyjson_val *)arr);
}

void *sx_yyjson_arr_get(void *arr, size_t idx)
{
    return (void *)yyjson_arr_get((yyjson_val *)arr, idx);
}

void *sx_yyjson_arr_get_first(void *arr)
{
    return (void *)yyjson_arr_get_first((yyjson_val *)arr);
}

void *sx_yyjson_arr_get_last(void *arr)
{
    return (void *)yyjson_arr_get_last((yyjson_val *)arr);
}

size_t sx_yyjson_obj_size(void *obj)
{
    return yyjson_obj_size((yyjson_val *)obj);
}

void *sx_yyjson_obj_get(void *obj, const char *key)
{
    return (void *)yyjson_obj_get((yyjson_val *)obj, key);
}

void *sx_yyjson_obj_getn(void *obj, const char *key, size_t key_len)
{
    return (void *)yyjson_obj_getn((yyjson_val *)obj, key, key_len);
}

bool sx_yyjson_obj_iter_init(void *obj, void *iter)
{
    return yyjson_obj_iter_init((yyjson_val *)obj, (yyjson_obj_iter *)iter);
}

void *sx_yyjson_obj_iter_next(void *iter)
{
    return (void *)yyjson_obj_iter_next((yyjson_obj_iter *)iter);
}

void *sx_yyjson_obj_iter_get_val(void *iter_val)
{
    return (void *)yyjson_obj_iter_get_val((yyjson_val *)iter_val);
}

void *sx_yyjson_read_opts(const char *dat, size_t len, uint32_t flg, void *alc, void *err)
{
    return (void *)yyjson_read_opts((char *)dat, len, (yyjson_read_flag)flg, (const yyjson_alc *)alc,
                                    (yyjson_read_err *)err);
}

void sx_yyjson_doc_free(void *doc)
{
    yyjson_doc_free((yyjson_doc *)doc);
}

void *sx_yyjson_doc_get_root(void *doc)
{
    return (void *)yyjson_doc_get_root((yyjson_doc *)doc);
}

char *sx_yyjson_write_opts(void *doc, uint32_t flg, void *alc, size_t *len, void *err)
{
    return yyjson_write_opts((yyjson_doc *)doc, (yyjson_write_flag)flg, (const yyjson_alc *)alc, len,
                             (yyjson_write_err *)err);
}
