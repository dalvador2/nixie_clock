module pipe(od, id,l){
    difference(){
        cylinder(h=l,r=od/2);
        translate([0,0,-1])cylinder(h=l+2,r=id/2);
        };
    }

module nipple(){
    cylinder(h=9,r2=0, r1=9/3);}

module nixie_tube(){
    cylinder(h=46,r=18/2);
    translate([0,0,46])nipple();}


module arm(
len_up = 65,
len_out = 40,
id = 19,
od = 22){

translate([0,0,len_up])nixie_tube();
pipe(od,id,len_up);
rotate([-90,0,0])pipe(od,id,len_out);}

arm();

