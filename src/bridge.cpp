// -*- coding: utf-8 -*-
// ==========================================================================
// 文件: bridge.cpp
// 创建者：陈云川/expnnchen
// 创建时间：2022/03/25 
//
// 描述: The purpose of this file is ...
// ===========================================================================

#include "bridge.h"

namespace bridge {

std::unique_ptr<Darts::DoubleArray> new_datrie() {
    return std::make_unique<Darts::DoubleArray>();
}

const unsigned int* get_array(const Darts::DoubleArray& da) {
    return (const unsigned int*)da.array();
}

void set_array(Darts::DoubleArray& da, const unsigned int* ptr, size_t size) {
    da.set_array((void*)ptr, size);
}

int build(Darts::DoubleArray& da, std::size_t num_keys,
          const char *const * keys,
          const std::size_t *lengths,
          const int *values) {
    try {
        return da.build(num_keys, keys, lengths, values);
    } catch (Darts::Details::Exception& e) {
        return -1;
    }
}

size_t common_prefix_search(const Darts::DoubleArray& da, const char* key, int* result,
                            size_t max_num_results, size_t length, size_t node_pos) {
    return da.commonPrefixSearch(key, result, max_num_results, length, node_pos);
}

}
