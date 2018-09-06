```
header : 10
  signature : 8 // "PSDDIFF1"
  version : 2 // BE u16 == 1
blocks: {...}
  block_{n} : {...}
    action: 2 // BE u16
              // 0 - skip
              // 1 - add
              // 2 - remove
              // 3 - replace
              // 4 - replace with same length
    # if action == 0 :
      data_length : 4 // BE u32
    # if action == 1 :
      data_length : 4 // BE u32
      data : data_length
    # if action == 2 :
      data_length : 4 // BE u32
    # if action == 3 :
      remove_length : 4 // BE u32
      data_length : 4 // BE u32
      data : data_length
    # if action == 4 :
      data_length : 4 // BE u32
      data : data_length
```
