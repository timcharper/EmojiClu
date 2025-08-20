#!/bin/bash
image_size=512
border_thickness=20

process_svgs() {
  mkdir -p emojis-phase1
  for set in {0..6}; do
    for n in {0..7}; do
      echo ./openmoji-svg-color/picks/$set/$n.svg
      inkscape -z --export-filename emojis-phase1/${set}-${n}.png -w 512 -h 512 ./openmoji-svg-color/picks/$set/$n.svg
    done
  done

  for other in left-of maybe-assertion negative-assertion; do
    echo $other
    
    inkscape -z --export-filename emojis-phase1/${other}.png -w 512 -h 512 ./openmoji-svg-color/picks/icons/${other}.svg
  done
}

# first phase, gradients... gonna do these mostly by hand. What border color should they have though? ROYGBIV ?

process_faces() {
  declare -a GRADIENTS
  declare -a BORDERS
  GRADIENTS=(
    "#FFFFFF-#FE54FF"
    "navy-cyan"
    "darkred-gold"
    "indigo-orange"
    "#000-lightgreen" # robot
    "#700-#f77" # party
    "#FFFFFF-#006FFF"
    "darkgrey-turquoise"
  )

  BORDERS=(
    "#61B2E4"  # matches halo color
    "#FF4500"  # OrangeRed for "navy-cyan" (strong pop against the cool tones)
    "#00FFFF"  # Cyan for "darkred-gold" (cool contrast to warm tones)
    "#00FF00"  # Lime Green for "indigo-orange" (electric and striking contrast)
    "#FFA500"  # Orange for "darkblue-lightpink" (complements and enhances contrast)
    "#8B0000"  # Party guy
    "#FF9000"  # Zary face
    "#FF00FF"  # Magenta for "darkgrey-turquoise" (vibrant pop against muted tones)
  )


  mkdir -p emojis-phase2
  n=0
  for gradient in "${GRADIENTS[@]}"; do
    border=${BORDERS[$n]}
    echo $gradient
    convert \
      \( -size 512x512 radial-gradient:${gradient} \) \
      \( emojis-phase1/0-${n}.png -modulate 80,150,100 \) -alpha set -background none -gravity center -composite \
      -bordercolor "${border}" -border 20 \
      emojis-phase2/0-${n}.png

    n=$((n + 1))
  done
}

process_others() {

  GRADIENTS=(
    "white-#dddddd"
    "grey-wheat"
    # "darkorange-goldenrod"   # Warm Orange gradient
    # "gold-olive"             # Subtle Yellow gradient
    "darkgrey-grey"         # Deep Red gradient
    "forestgreen-white"   # Natural Green gradient
    "steelblue-darkblue"         # Cool Blue gradient
    "indigo-slateblue"       # Deep Purple gradient
    # "violet-plum"            # Soft Magenta gradient
  )

  BORDERS=(
    "darkgrey"
    "chocolate"
    # "#0000FF"  # Blue for "darkorange-goldenrod" (strong contrast to orange)
    "#550000"  # Cyan for "darkred-maroon" (cool contrast to warm red tones) # "#8B008B"  # Dark Magenta for "gold-olive" (adds depth to the yellow-green mix)
    "#FF1493"  # Deep Pink for "forestgreen-seagreen" (bright and playful contrast)
    "#FF8C00"  # Dark Orange for "teal-steelblue" (warm against the cool background)
    "#00FF00"  # Lime Green for "indigo-slateblue" (high-energy contrast)
    "#FFD700"  # Gold for "violet-plum" (brightens and complements the purple tones)
  )

  n=0
  img_n=1
  ONLY=${ONLY:--1}
  for gradient in "${GRADIENTS[@]}"; do
    echo $ONLY
    if [ $ONLY -eq -1 ] || [ $img_n -eq $ONLY ]; then
      border=${BORDERS[$n]}
      for i in {0..7}; do
      convert \
          \( -size 512x512 gradient:"${gradient}" -define gradient:angle=45 -define gradient:vector="0,0 512,512" \) \
          emojis-phase1/${img_n}-${i}.png -modulate 100,150,100 -alpha set -background none -gravity center -composite \
          -bordercolor "${border}" -border 20 \
          emojis-phase2/${img_n}-${i}.png
      done
    fi
    n=$((n + 1))
    img_n=$((img_n + 1))
  done
}

movein() {
  # 0:0
  # 1:1
  # numbers:2
  # 3:3
  # 5:4
  # letters:5
  # 2:6
  # 4:7

  for i in {0..7}; do
    case $i in
      0) prefix=emojis-phase2/0-;;
      1) prefix=emojis-phase2/1-;;
      2) prefix=numbers/;;
      3) prefix=emojis-phase2/3-;;
      4) prefix=emojis-phase2/5-;;
      5) prefix=letters/;;
      6) prefix=emojis-phase2/2-;;
      7) prefix=emojis-phase2/4-;;
    esac

    for j in {0..7}; do
      cp ${prefix}${j}.png ../resources/assets/icons/${i}/${j}.png
    done

    cp emojis-phase2/maybe-assertion.png ../resources/assets/icons/maybe-assertion.png
    cp emojis-phase2/negative-assertion.png ../resources/assets/icons/negative-assertion.png
  done
}

process_symbols() {
  
  # convert -size ${image_size}x${image_size} gradient:${gradient} \
  #         -fill none -stroke ${color} -strokewidth ${border_thickness} \
  #         -draw "rectangle 0,0 $((image_size-1)),$((image_size-1))" \
  #         -fill "${color}" -stroke none -weight 700 -font Liberation-Sans-Bold -pointsize 600 -gravity center \
  #         -annotate +0+0 "${number}" \
  #         ${output_image}
  # number=$((number + 1))



  # maybe-assertion
  # gradient="white-#dddddd"
  # image_name="maybe-assertion"
  # border="yellow"
  # convert \
  #     \( -size ${image_size}x${image_size} gradient:"${gradient}" -define gradient:angle=45 -define gradient:vector="0,0 512,512" \) \
  #     emojis-phase1/${image_name}.png -modulate 100,150,130 -alpha set -background none -gravity center -composite \
  #     -bordercolor "${border}" -border ${border_thickness} \
  #     emojis-phase2/${image_name}.png

  # negative-assertion
  # image_name="negative-assertion"
  # border="red"
  # gradient="black-grey"
  # convert \
  #     emojis-phase1/${image_name}.png \
  #     \( +clone -background black -shadow 80x15+25+25 \) \
  #     +swap -background none -layers merge \
  #     -gravity northwest -extent ${image_size}x${image_size} \
  #     emojis-phase2/${image_name}.png

  # left-of
  image_name="left-of"
  border="black"
  gradient="yellow-yellow"
  convert \
      \( -size 512x512 gradient:${gradient} \) \
      \( emojis-phase1/${image_name}.png -modulate 500,150,100 \) -alpha set -background none -gravity center -composite \
      -bordercolor "${border}" -border 20 \
      emojis-phase2/${image_name}.png
}

case "$1" in
  svgs) process_svgs;;
  faces) process_faces;;
  others) process_others;;
  symbols) process_symbols;;
  movein) movein;;
  *) echo "Usage: $0 svgs|faces|others|movein";;
esac
