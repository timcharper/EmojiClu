#!/bin/bash

# Image size and border thickness
image_size=512
border_thickness=26

# Numbers

gradient="black-#222222"
# color="#FFFFFF"
color="#09FB05"

mkdir -p numbers
number=1
for n in {0..7}; do
    output_image="numbers/${n}.png"
    convert -size ${image_size}x${image_size} gradient:${gradient} \
            -fill none -stroke ${color} -strokewidth ${border_thickness} \
            -draw "rectangle 0,0 $((image_size-1)),$((image_size-1))" \
            -fill "${color}" -stroke none -weight 700 -font Liberation-Sans-Bold -pointsize 600 -gravity center \
            -annotate +0+0 "${number}" \
            ${output_image}
    number=$((number + 1))
done

# Letters