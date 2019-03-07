# Abode Photoshop® PSD format interpretation based on Official Adobe PSD Specification:
[Official Abobe Photoshop® PSD specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)

```
big_endian

header : 26
  signature: ascii 4 // "8BPS"
  version: uint16    // [0x0001] - psd
                     // [0x0002] - psb
  reserved: bytes 6  // must be zero
  number_of_channels: uint16
  image_height: uint32
  image_width: uint32
  depth: uint16
  color_mode: uint32 // [0x0000] - Bitmap
	                   // [0x0001] - Grayscale
	                   // [0x0002] - Indexed
	                   // [0x0003] - RGB
	                   // [0x0004] - CMYK
	                   // [0x0007] - Multichannel
	                   // [0x0008] - Duotone
	                   // [0x0009] - LAB
color_mode_section_length : uint32
color_mode_section : *color_mode_section_length
image_resources_length : uint32
image_resources : *image_resources_length
  # while pos() < pos(image_resources) + *image_resources_length
    image_resource_{n} : {...}
      signature : ascii 4 // 8BIM
      id : uint16
      name_length : uint8
      name : name_length == 0 ? 1 : pad(*name_length + 1, 2) - 1
      data_length : uint32
      data : pad(*data_length, 2)
layers_resources_length : (uint32 | uint64) psd | psb accordingly
layers_resources : *layers_resources_length
  layers_info_length : (uint32 | uint64) psd | psb accordingly
  layers_info : *layers_info_length
    layer_count : *layers_info_length == 0 ? 0 | int16 // If it is a negative number,
                                                       // its absolute value is the number of layers and
				                                               // the first alpha channel contains
                                                       // the transparency data for the merged result.
    # for i = 0; i < *layer_count; i++
      layer_{n} : {...}
        rect : 16
          top: int32
          left: int32
          bottom: int32
          right: int32
        channel_info : 2 + (((4 | 8) psd | psb accordingly) * */layers_resources/layers_info/layer_{n}/channel_info/number)
          number : uint16
          channel_{n} : (6 | 10) psd | psb accordingly
            id : int16 // negative if channel is mask
            length : (uint32 | uint64) psd | psb accordingly
        blend_mode_signature : ascii 4 // 8BIM
        blend_mode_key : ascii 4
        opacity : uint8
        clipping : uint8
        flags : uint8
        filler : uint8
        extra_data_length : uint32
        extra_data : pad(*extra_data_length, 2)
          mask_data_length : uint32
          mask_data : *mask_data_length
          blending_ranges_length : uint32
          blending_ranges : {...}
            blending_range_{n} : uint32 // Composite gray blend source
            blending_range_{n} : uint32 // Composite gray blend destination range
            blending_range_{n} : uint32 // n channel source range
            blending_range_{n} : uint32 // n channel destination range
                                        // ...
          name_length : uint8
          name : pad(*name_length + 1, 4) - 1
          additional_data : {pos()...pos(extra_data) + *extra_data_length}
    channel_data : {...}
      # for i = 0; i < count(/layers/resources/layers_info/layer_{n}); i++
        layer_{n} : {...}
          # for i = 0; i < */layers/resources/layers_info/layer_{n}/channel_info/number; i++
          channel_{n} : {...}
            compression_method : uint16 // [0x0000] - Uncompressed
                                        // [0x0001] - RLE
                                        // [0x0002] - ZIP with prediction
                                        // [0x0003] - ZIP withou prediction
            data : */layers/resources/layers_info/layer_{n}/channel_info/channel_{n}/length - 2
    padding : (pos(layers_info) + *layers_info_length) - pos()
  global_mask_length : uint32
  global_mask : *global_mask_length
  additional_layer_information : {...pos(layers_resources) + *layers_resources_length}
image_data : {...}
  compression_method : uint16 // [0x0000] - Uncompressed
                              // [0x0001] - RLE
                              // [0x0002] - ZIP with prediction
                              // [0x0003] - ZIP withou prediction
  data : {...EOF}
```
