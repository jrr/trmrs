#[derive(Copy, Clone)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl std::fmt::Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl Dimensions {
    pub fn fits_in(&self, other: &Dimensions) -> bool {
        self.width <= other.width && self.height <= other.height
    }
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions_display() {
        let dim = Dimensions {
            width: 1920,
            height: 1080,
        };
        assert_eq!(format!("{}", dim), "1920x1080");
    }

    #[test]
    fn test_fits_in_true() {
        let small = Dimensions {
            width: 800,
            height: 600,
        };
        let large = Dimensions {
            width: 1920,
            height: 1080,
        };
        assert!(small.fits_in(&large));
    }

    #[test]
    fn test_fits_in_false() {
        let large = Dimensions {
            width: 1920,
            height: 1080,
        };
        let small = Dimensions {
            width: 800,
            height: 600,
        };
        assert!(!large.fits_in(&small));
    }

    #[test]
    fn test_fits_in_equal() {
        let dim1 = Dimensions {
            width: 1024,
            height: 768,
        };
        let dim2 = Dimensions {
            width: 1024,
            height: 768,
        };
        assert!(dim1.fits_in(&dim2));
    }

    #[test]
    fn test_fits_in_partial() {
        let dim1 = Dimensions {
            width: 800,
            height: 1200,
        };
        let dim2 = Dimensions {
            width: 1000,
            height: 800,
        };
        assert!(!dim1.fits_in(&dim2));
        assert!(!dim2.fits_in(&dim1));
    }

    #[test]
    fn test_area() {
        let dim = Dimensions {
            width: 10,
            height: 20,
        };
        assert_eq!(dim.area(), 200);
    }

    #[test]
    fn test_fmt() {
        let dim = Dimensions {
            width: 10,
            height: 20,
        };
        assert_eq!(format!("{}", dim), "10x20");
    }
}
