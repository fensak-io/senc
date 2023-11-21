// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

// senc specific objects

((globalThis) => {
  const is_senc_out_data = (a) => {
    return typeof a.__is_senc_out_data === 'function'
  }
  const assert_all_items_are_senc_out_data = (arr) => {
    for (const a of arr) {
      if (!is_senc_out_data(a)) {
        throw new Error("OutDataArray element must be an OutData object");
      }
    }
  }

  /**
   * Special class that indicates to senc that there is output metadata.
   * This is useful to control the output behavior of the IaC objects, such as specifying the file format and extension.
   */
  class OutData {
    constructor(attrs) {
      this.out_path = attrs.out_path;
      this.out_ext = attrs.out_ext;
      this.out_type = attrs.out_type;
      this.out_prefix = attrs.out_prefix
      this.data = attrs.data;
    }

    /**
     * A special marker function to indicate this is a senc OutData object to the runtime.
     */
    __is_senc_out_data() {}
  }

  /**
   * A list of OutData objects. We use a class instead of a type so that we can bind a function that indicates this is
   * an OutDataArray to the runtime.
   */
  class OutDataArray extends Array {
    constructor(...args) {
      assert_all_items_are_senc_out_data(args)
      super(...args)
    }

    push(...args) {
      assert_all_items_are_senc_out_data(args)
      return super.push(...args)
    }

    /**
     * A special marker function to indicate this is a senc OutDataArray object to the runtime.
     */
    __is_senc_out_data_array() {}
  }

  globalThis.senc = {
    OutData: OutData,
    OutDataArray: OutDataArray,
  };
})(globalThis);
