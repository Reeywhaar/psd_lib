# psd_spec
## Abode Photoshop PSD format interpretation based on Official Adobe PSD Specification:
https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/

```
header : 26
  signature: 4 // "8BPS" in ASCII
  version: 2
  reserved: 6 // must be zero
  number_of_channels: 2
  image_height: 4
  image_width: 4
  depth: 2
  color_mode: 2
color_mode_section_length : 4
color_mode_section : *color_mode_section_length
image_resources_length : 4
image_resources : *image_resources_length
  # while pos() < pos() + *image_resources_length
    image_resource_{n} : {...}
      signature : 4
      id : 2
      name_length : 1
      name : name_length == 0 ? 1 : pad(*name_length, 2) - 1
      data_length : 4
      data : *data_length
layers_resources_length : 4
layers_resources : *layers_resources_length
  layers_info_length : 4
  layers_info : *layers_info_length
    layer_count : 2
    # for i = 0; i < &layer_count; i++
      layer_{n} : {...}
        rect : 16
          top: 4
          left: 4
          bottom: 4
          right: 4
        channel_info : 6 * */layers_resources/layers_info/layer_{n}/channel_info/number
          number : 2
          channel_{n} : 6
            id : 2
            length : 4
        blend_mode_signature : 4
        blend_mode_key : 4
        opacity : 1
        clipping : 1
        flags : 1
        filler : 1
        extra_data_length : 4
        extra_data : *extra_data_length
          mask_data_length : 4
          mask_data : *mask_data_length
          blending_ranges_length : 4
          blending_ranges : {...}
            blending_range_{n} : 4 // Composite gray blend source
            blending_range_{n} : 4 // Composite gray blend destination range
            blending_range_{n} : 4 // n channel source range
            blending_range_{n} : 4 // n channel destination range
          name_length : 1
          name : pad(*name_length, 4) - 1
          additional_data : {...pos(extra_data) + *extra_data_length}
    channel_data : {...}
      # for i = 0; i < count(/layers/resources/layers_info/layer_{n}); i++
        layer_{n} : {n}
          # for i = 0; i < */layers/resources/layers_info/layer_{n}/channel_info/number; i++
          channel_{n} : {...}
            compression_method : 2
            data : */layers/resources/layers_info/layer_{n}/channel_info/channel_{n}/length - 2
  padding : (pos(layers_info) + *layers_info_length) - pos()
  global_mask_length : 4
  global_mask : *global_mask_length
  additional_layer_information : {...pos(layers_resources) + *layers_resources_length}
image_data : {...}
  compression_method : 2
  data : {...EOF}
```
