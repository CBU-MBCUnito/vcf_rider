initSidebarItems({"struct":[["Matrix","Struct used to represent PWM likelihoods/nucleotide frequencies.  ncols is used to be able to represent 4 and 5 nucleotides PWMs (to manage N in sequences). No consistency checks are performed for get/set operations for efficiency."],["PWM","Struct representing a whole Positional Weight Matrix."],["PWMReader","Struct used to read PWM. It will be implement an Iterator of `PWM` structs. The String buffer is used to get characters from the `BufReader` one by one and load the `freq` for the loaded `PWM` structs. The expected format is the same used by matrix_rider in C, therefore a tab delimited file with these columns: pwm_name, pos, count_a, count_c, count_g, count_t pos is ignored, count_x should be positive integer different from 0."]]});