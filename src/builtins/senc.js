// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

// senc specific objects

((globalThis) => {
  // Special class that indicates to senc that there is output metadata.
  // This is useful to control the output behavior of the IaC objects, such as specifying the file format and extension.
  class OutData {
    constructor(out_ext, out_type, data) {
      this.out_ext = out_ext;
      this.out_type = out_type;
      this.data = data;
    }

    __is_senc_out_data() {}
  }

  globalThis.senc = {
    OutData: OutData,
  };
})(globalThis);
