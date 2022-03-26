// -*- coding: utf-8 -*-
// ==========================================================================
// 文件: bridge.h
// 创建者：陈云川/expnnchen
// 创建时间：2022/03/25 
//
// 描述: The purpose of this file is ...
// ===========================================================================

#pragma once

#ifndef DARTS_CLONE_RS_BRIDGE_H_
#define DARTS_CLONE_RS_BRIDGE_H_

#include <memory>
#include "darts.h"

namespace bridge {

std::unique_ptr<Darts::DoubleArray> new_datrie();


const unsigned int* get_array(const Darts::DoubleArray& da);

void set_array(Darts::DoubleArray& da, const unsigned int* ptr, size_t size);

int build(Darts::DoubleArray& da, std::size_t num_keys,
          const char* const* keys,
          const std::size_t *lengths = nullptr,
          const int *values = nullptr);

size_t common_prefix_search(const Darts::DoubleArray& da, const char* key, int* result,
                            size_t max_num_results, size_t length, size_t node_pos);

}



#endif // DARTS_CLONE_RS_BRIDGE_H_
