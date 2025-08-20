#!/bin/bash

# Image size and border thickness
image_size=512
border_thickness=26


# identify -list font | grep Bold | grep -v Italic | grep -v glyphs | grep -v family | sed "s/ *Font: *//" | while read font; do
#     output_image="choices/${font}.png"

#     echo "Font: ${font}"
#     # Create a black background with a bright green border and a bold, large green number 1
#     convert -size ${image_size}x${image_size} xc:black \
#             -fill none -stroke green -strokewidth ${border_thickness} \
#             -draw "rectangle 0,0 $((image_size-1)),$((image_size-1))" \
#             -fill green -stroke none -weight 700 -font $font -pointsize 600 -gravity center \
#             -annotate +0+0 "4" \
#             ${output_image}

#     echo "Image created: ${output_image}"

# done


# Letters

mkdir -p letters
n=0
for letter in E M O J I C L U; do
    font="NimbusMonoPS-Bold"
    gradient="white-#FFFFDD"
    # bright blue 0000ff
    border_color="#0000ff"
    letter_color="#0000ff"
    output_image="letters/${n}.png"
    convert -size ${image_size}x${image_size} radial-gradient:${gradient} \
            -fill none -stroke ${border_color} -strokewidth ${border_thickness} \
            -draw "rectangle 0,0 $((image_size-1)),$((image_size-1))" \
            -fill ${letter_color} -stroke none -weight 700 -font ${font} -pointsize 600 -gravity center \
            -annotate +0+110 "${letter}" \
            ${output_image}
    n=$((n+1))
done