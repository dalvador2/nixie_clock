use <nixie_arm.scad>

module field(spacing = 35){
    arm();
    translate([spacing,0,0])arm();}

module array(spacing=85, units=3){
    for(i = [0:units-1]){
    translate([i*spacing,0,0])field();}
    }

array();