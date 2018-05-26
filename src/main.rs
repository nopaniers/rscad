//==============================================================================
// Rust CAD
//==============================================================================
//
// Tool for procedurally generating CAD drawings. Aims to
//
//   - Reproduce OpenSCAD functions
//   - Read/Write STL files
//
//==============================================================================

use std::ops::{Add, Sub, Neg, Div, AddAssign, MulAssign};
use std::f32::consts::{PI};
use std::fs::File;

extern crate itertools;
use itertools::zip;

extern crate byteorder;
use std::io::{Result, Write};
use byteorder::{LittleEndian, WriteBytesExt};

use std::fmt;



//==============================================================================
// Options
//==============================================================================

// const fa: Length = 0.0; // minimum angle
// const fs: Length = 0.0; // minimum size
const FRAGMENTS: u32 = 32;     // number of fragments



//==============================================================================
// Geometry
//==============================================================================

type Length = f32;


//------------------------------------------------------------------------------
// Points
//------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Point {
   x: Length,
   y: Length,
   z: Length
}

type Vector = Point;

static ORIGIN: Point = Point {x: 0.0, y: 0.0, z: 0.0};



impl Point {
   /// Create a new Point with coordinates `x` `y` and `z`.
   fn new(x: Length, y: Length, z: Length) -> Point {
      Point{x: x, y: y, z: z}
   }
   
   /// Dot product
   fn dot(&self, other: Point) -> Length {
      self.x * other.x + self.y * other.y + self.z * other.z
   }

   /// Cross product
   fn cross(self, other: Point) -> Point {
      Point{x: self.y*other.z - self.z*other.y,
            y: self.z*other.x - self.x*other.z,
            z: self.x*other.y - self.y*other.x}
   }

   fn length(self) -> Length {
      (self.x*self.x + self.y*self.y + self.z*self.z).sqrt()
   }

   fn unit_vector(self) -> Vector {
      self/self.length()
   }
}



impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
}

impl AddAssign for Point {
   
    fn add_assign(&mut self, other: Point) {
       self.x += other.x;
       self.y += other.y;
       self.z += other.z;
    }
}



impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {x: self.x - other.x, y: self.y - other.y, z: self.z - other.z}
    }
}


impl Div<Length> for Point {
   type Output = Point;
   
   fn div(self, scale: Length) -> Point {
      assert!(scale!=0.0);
      Point {x: self.x/scale, y: self.y/scale, z: self.z/scale}
   }
}

impl MulAssign<Length> for Point {

   fn mul_assign(&mut self, scale: Length) {
      self.x *= scale;
      self.y *= scale;
      self.z *= scale;
   }
}


impl Neg for Point {
    type Output = Point;

    fn neg(self) -> Point {
       Point {x: -self.x,
              y: -self.y,
              z: -self.z}
    }
}



impl fmt::Display for Point
{
   // Display a Block in text output
   fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {

      write!(fmt, "{x:e} {y:e} {z:e}", x=self.x, y=self.y, z=self.z)?;
      
      Ok(())
   }
}





//------------------------------------------------------------------------------
// Triangles
//------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Triangle {
   vertex: [Point; 3]
}


impl Triangle {

   fn new(p1: Point, p2: Point, p3: Point) -> Triangle {
      Triangle{ vertex: [p1, p2, p3] }
   }
}



impl Add<Vector> for Triangle {
   type Output = Triangle;

   fn add(self, vector: Vector) -> Triangle {
      let mut tri = self.clone();
      for vertex in &mut tri.vertex {
         *vertex += vector;
      }
      tri
   }
}


impl AddAssign<Vector> for Triangle {
   fn add_assign(&mut self, vector: Vector) {
      for vertex in &mut self.vertex {
         *vertex += vector;
      }
   }
}

impl MulAssign<Length> for Triangle {
   fn mul_assign(&mut self, scale: Length) {
      for vertex in &mut self.vertex {
         *vertex *= scale;
      }
   }
}


impl fmt::Display for Triangle
{
   // Display a Block in text output
   fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {

      write!(fmt, "{p1} -- {p2} -- {p3}", p1=self.vertex[0],
             p2=self.vertex[1], p3=self.vertex[2])?;
      
      Ok(())
   }
}



//------------------------------------------------------------------------------
// Objects
//------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Face {
   normal: Vector,
   triangle: Triangle,
   colour: Colour
}


#[derive(Clone, Copy)]
struct Colour {
   r: u8,
   g: u8,
   b: u8,
   alpha: u8
}

#[derive(Clone)]
struct Object {
   faces: Vec<Face>
}




impl Object {

   //---------------------------------------------------------------------------
   // Basic Objects
   //---------------------------------------------------------------------------

   fn umbrella(centre: &Point, spokes: Vec<Point>) -> Object {
      let mut obj = Object::new();
      
      for (p2, p3) in zip(&vertices, &vertices[1..]) {
         obj += Triangle::new(centre, *p2, *p3);
      }
      obj += Triangle::new(centre, vertices[0], vertices[-1]);
      
      obj
   }
   
   fn icosahedron(radius: Length) -> Object {
      // From Wikipedia:
      //
      // The locations of the vertices of a regular icosahedron can be
      // described using spherical coordinates, for instance as
      // latitude and longitude. If two vertices are taken to be at
      // the north and south poles (latitude ±90°), then the other ten
      // vertices are at latitude ±arctan(1/2) ≈ ±26.57°. These ten
      // vertices are at evenly spaced longitudes (36° apart),
      // alternating between north and south latitudes.

      let obj = Object::new();
      use Object::umbrella;
      
      let lat = 0.5_f32.atan();
      let theta = PI/5.0;

      obj += umbrella(Point{x: 0.0, y: 0.0, z: 1.0},
                      (0..5).map(|i| Point{x: (theta*i as f32).cos(),
                                           y: (theta*i as f32).sin(),
                                           z: lat}).collect::Vec<Point>());
      

      obj
   }
   
   fn sphere(radius: Length) {

   }

   
   fn cube(size: Length, center: Point) -> Object {
      Object::rectangular_prism(size, size, size)
   }

   
   fn rectangular_prism(width: Length, depth: Length, height: Length) -> Object
   {
      let mut obj = Object::new();
      
      let vx = Point{x: width, y: 0.0, z: 0.0};
      let vy = Point{x: 0.0, y: depth, z: 0.0};
      let vz = Point{x: 0.0, y: 0.0, z: height};

      let bottom = Shape::squarish(vx, vy);
      obj += &bottom;
      obj += bottom + vz;

      let side = Shape::squarish(vy, vz);
      obj += &side;
      obj += side + vx;

      let front = Shape::squarish(vx, vz);
      obj += &front;
      obj += front + vy;
      
      obj
   }

   
   fn cylinder(height: Length, radius: Length) -> Object {
      let mut obj = Shape::circle(radius);
      // obj.extrude(height);
      obj
   }

   
   fn polyhedron(points: Vec<Point>) {}


   //----------------------------------------------------------------------------
   // Transformations of Objects
   //----------------------------------------------------------------------------

   fn translate(&mut self, by: &Vector) {
      *self += *by;
   }

   
   fn rotate(self, by: Vector) {}
   
   fn scale(&mut self, factor: f32) {
      *self *= factor;
   }


   fn resize(&self, size: Point) {}
   fn mirror(&mut self, around: Point) {}
   // fn multmatrix() {}
   fn color_by_name(&self, colour_name: String, alpha: f32) {}
   fn color(&self, r: f32, g: f32, b: f32, a: f32) {}
   fn offset(&self, r: Length) {}
   fn hull(&self, other: &Object) {}
   fn minkowski(&self, other: &Object) {}  

}



impl<'a> AddAssign<&'a Object> for Object {
   fn add_assign(&mut self, other: &'a Object) {
      self.faces.extend(&other.faces);
   }
}

impl AddAssign<Object> for Object {
   fn add_assign(&mut self, other: Object) {
      self.faces.extend(other.faces);
   }
}



impl AddAssign<Triangle> for Object {
   fn add_assign(&mut self, triangle: Triangle) {
      let face = Face {
         normal: Vector {x: 0.0, y: 0.0, z: 1.0},
         triangle: triangle,
         colour: Colour {r: 0, g: 0, b: 0, alpha: 0}
      };
      self.faces.push(face);
   }
}



impl Add<Vector> for Object {
   type Output = Object;

   fn add(self, vector: Vector) -> Object {
      let mut obj = self.clone();
      for face in &mut obj.faces {
         face.triangle += vector;
      }
      obj
   }
}

impl AddAssign<Vector> for Object {
   fn add_assign(&mut self, vector: Vector) {
      for face in &mut self.faces {
         face.triangle += vector;
      }  
   }
}

impl MulAssign<Length> for Object {
   fn mul_assign(&mut self, scale: Length) {
      for face in &mut self.faces {
         face.triangle *= scale;
      }  
   }
}




//------------------------------------------------------------------------------
// 2D Shapes
//------------------------------------------------------------------------------


type Shape = Object;


impl Shape {

   fn new() -> Shape {
      Shape{ faces: Vec::new() }
   }

   
   fn circle(r: Length) -> Shape {
      let angle = 2.0*PI/(FRAGMENTS as f32);
      let mut points = Vec::new();
      
      for index in 0..FRAGMENTS {
         let theta = index as f32 * angle;
         points.push(Point::new(r*theta.cos(), r*theta.sin(), 0.0));
      }

      Shape::polygon(points)
   }

   
   fn squarish(v1: Vector, v2: Vector) -> Shape {
      let points = vec!{ORIGIN, v1, v1+v2, v2};
      
      Shape::polygon(points)
   }

   
   fn square(l: Length) -> Shape
   {
      Shape::rectangle(l, l)
   }


   fn rectangle(width: Length, height: Length) -> Shape
   {
      Shape::squarish(Vector{x: width, y: 0.0, z: 0.0},
                      Vector{x: 0.0, y: height, z: 0.0})
   }

   fn polygon(vertices: Vec<Point>) -> Shape  {
      let mut shape = Shape::new();
      let p1 = vertices[0];
      
      for (p2, p3) in zip(&vertices[1..], &vertices[2..]) {
         shape += Triangle::new(p1, *p2, *p3);
      }
      
      shape
   }
   
   fn icosahedron(radius: Length) -> Object {
      // From Wikipedia:
      //
      // The locations of the vertices of a regular icosahedron can be
      // described using spherical coordinates, for instance as
      // latitude and longitude. If two vertices are taken to be at
      // the north and south poles (latitude ±90°), then the other ten
      // vertices are at latitude ±arctan(1/2) ≈ ±26.57°. These ten
      // vertices are at evenly spaced longitudes (36° apart),
      // alternating between north and south latitudes.

      let obj = Object::new();
      use Object::umbrella;
      
      let lat = 0.5_f32.atan();
      let theta = PI/5.0;

      obj += umbrella(Point{x: 0.0, y: 0.0, z: 1.0},
                      (0..5).map(|i| Point{x: (theta*i as f32).cos(),
                                           y: (theta*i as f32).sin(),
                                           z: lat}).collect::Vec<Point>());
      

      obj
   }
   
   fn sphere(radius: Length) {

   }

   
   fn cube(size: Length, center: Point) -> Object {
      Object::rectangular_prism(size, size, size)
   }

   
   fn rectangular_prism(width: Length, depth: Length, height: Length) -> Object
   {
      let mut obj = Object::new();
      
      let vx = Point{x: width, y: 0.0, z: 0.0};
      let vy = Point{x: 0.0, y: depth, z: 0.0};
      let vz = Point{x: 0.0, y: 0.0, z: height};

      let bottom = Shape::squarish(vx, vy);
      obj += &bottom;
      obj += bottom + vz;

      let side = Shape::squarish(vy, vz);
      obj += &side;
      obj += side + vx;

      let front = Shape::squarish(vx, vz);
      obj += &front;
      obj += front + vy;
      
      obj
   }

   
   fn cylinder(height: Length, radius: Length) -> Object {
      let mut obj = Shape::circle(radius);
      // obj.extrude(height);
      obj
   }

   
   fn polyhedron(points: Vec<Point>) {}


   //----------------------------------------------------------------------------
   // Transformations of Objects
   //----------------------------------------------------------------------------

   fn translate(&mut self, by: &Vector) {
      *self += *by;
   }

   
   fn rotate(self, by: Vector) {}
   
   fn scale(&mut self, factor: f32) {
      *self *= factor;
   }


   fn resize(&self, size: Point) {}
   fn mirror(&mut self, around: Point) {}
   // fn multmatrix() {}
   fn color_by_name(&self, colour_name: String, alpha: f32) {}
   fn color(&self, r: f32, g: f32, b: f32, a: f32) {}
   fn offset(&self, r: Length) {}
   fn hull(&self, other: &Object) {}
   fn minkowski(&self, other: &Object) {}  

}



impl<'a> AddAssign<&'a Object> for Object {
   fn add_assign(&mut self, other: &'a Object) {
      self.faces.extend(&other.faces);
   }
}

impl AddAssign<Object> for Object {
   fn add_assign(&mut self, other: Object) {
      self.faces.extend(other.faces);
   }
}



impl AddAssign<Triangle> for Object {
   fn add_assign(&mut self, triangle: Triangle) {
      let face = Face {
         normal: Vector {x: 0.0, y: 0.0, z: 1.0},
         triangle: triangle,
         colour: Colour {r: 0, g: 0, b: 0, alpha: 0}
      };
      self.faces.push(face);
   }
}



impl Add<Vector> for Object {
   type Output = Object;

   fn add(self, vector: Vector) -> Object {
      let mut obj = self.clone();
      for face in &mut obj.faces {
         face.triangle += vector;
      }
      obj
   }
}

impl AddAssign<Vector> for Object {
   fn add_assign(&mut self, vector: Vector) {
      for face in &mut self.faces {
         face.triangle += vector;
      }  
   }
}

impl MulAssign<Length> for Object {
   fn mul_assign(&mut self, scale: Length) {
      for face in &mut self.faces {
         face.triangle *= scale;
      }  
   }
}




//------------------------------------------------------------------------------
// 2D Shapes
//------------------------------------------------------------------------------


type Shape = Object;


impl Shape {

   fn new() -> Shape {
      Shape{ faces: Vec::new() }
   }

   
   fn circle(r: Length) -> Shape {
      let angle = 2.0*PI/(FRAGMENTS as f32);
      let mut points = Vec::new();
      
      for index in 0..FRAGMENTS {
         let theta = index as f32 * angle;
         points.push(Point::new(r*theta.cos(), r*theta.sin(), 0.0));
      }

      Shape::polygon(points)
   }

   
   fn squarish(v1: Vector, v2: Vector) -> Shape {
      let points = vec!{ORIGIN, v1, v1+v2, v2};
      
      Shape::polygon(points)
   }

   
   fn square(l: Length) -> Shape
   {
      Shape::rectangle(l, l)
   }


   fn rectangle(width: Length, height: Length) -> Shape
   {
      Shape::squarish(Vector{x: width, y: 0.0, z: 0.0},
                      Vector{x: 0.0, y: height, z: 0.0})
   }

   fn polygon(vertices: Vec<Point>) -> Shape {
      let mut shape = Shape::new();
      let p1 = vertices[0];
      
      for (p2, p3) in zip(&vertices[1..], &vertices[2..]) {
         shape += Triangle::new(p1, *p2, *p3);
      }
      
      shape
   }

   fn text(text: String) {
   }

}


impl fmt::Display for Shape
{
   // Display a Block in text output
   fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {

      for face in &self.faces {
         write!(fmt, "{face}\n", face = face.triangle)?;
      }
      
      Ok(())
   }
}








//==============================================================================
// STL Files
//==============================================================================

fn write_point(mut file: &File, point: &Point) -> Result<()>
{
   file.write_f32::<LittleEndian>(point.x)?;
   file.write_f32::<LittleEndian>(point.y)?;
   file.write_f32::<LittleEndian>(point.z)?;
   Ok(())
}

fn write_stl(filename: &str, obj: &Object)  -> std::io::Result<()>
{
   // Write the header
   let mut buffer = File::create(filename)?;

   buffer.write_all(&[0_u8; 80])?;
   buffer.write_u32::<LittleEndian>(obj.faces.len() as u32)?;
   
   // Write the triangles' vertices
   for face in &obj.faces {
      write_point(&buffer, &face.normal)?;
      for vertex in face.triangle.vertex.iter() {
         write_point(&buffer, &vertex)?;
      }
      buffer.write_u16::<LittleEndian>(0)?;
   }
   
   Ok(())
}

fn read_stl(filename: String) -> Option<Object>
{
   None
}

fn write_text_stl(filename: &str, obj: &Object) -> std::io::Result<()>
{
   let mut buffer = File::create(filename)?;

   writeln!(buffer, "solid object")?;
   
   for face in &obj.faces {
      writeln!(buffer, "facet normal {normal}", normal=face.normal)?;
      writeln!(buffer, "  outer loop")?;
      for point in face.triangle.vertex.iter() {
         writeln!(buffer, "    vertex {point}", point=point)?;
      }
      writeln!(buffer, "  endloop")?;
      writeln!(buffer, "endfacet")?;
   }
   
   writeln!(buffer, "endsolid object")?;
   
   Ok(())
}

fn read_text_stl(filename: String) -> Option<Object>
{
   None
}



//==============================================================================
// Main
//==============================================================================

fn main() {
   println!("Rust CAD v.0.1");
   println!("==============");
 
   let rectangle = Shape::rectangle(10.0, 20.0);
   println!("{}", rectangle);
   write_text_stl("test.stl", &rectangle);

   
   let circle = Object::icosahedron(10.0);
   println!("{}", circle);
   write_stl("test_bin.stl", &circle);

}
