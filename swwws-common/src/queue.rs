use std::collections::VecDeque;
use std::path::PathBuf;
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sorting {
    Random,
    Ascending,
    Descending,
}

impl std::fmt::Display for Sorting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sorting::Random => write!(f, "random"),
            Sorting::Ascending => write!(f, "ascending"),
            Sorting::Descending => write!(f, "descending"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Queue {
    buffer: VecDeque<PathBuf>,
    current: Option<PathBuf>,
    tail: VecDeque<PathBuf>,
    size: usize,
    sorting: Sorting,
    images: Vec<PathBuf>,
}

impl Queue {
    pub fn new(size: usize, sorting: Sorting, images: Vec<PathBuf>) -> Option<Self> {
        if images.is_empty() {
            return None;
        }

        let mut queue = Self {
            buffer: VecDeque::new(),
            current: None,
            tail: VecDeque::new(),
            size,
            sorting,
            images: images.clone(),
        };

        queue.initialize(images);
        Some(queue)
    }

    fn initialize(&mut self, mut images: Vec<PathBuf>) {
        match self.sorting {
            Sorting::Random => {
                let mut rng = rand::thread_rng();
                images.shuffle(&mut rng);
            }
            Sorting::Ascending => {
                images.sort();
            }
            Sorting::Descending => {
                images.sort_by(|a, b| b.cmp(a));
            }
        }

        self.images = images;
        
        // Set the first image as current
        if !self.images.is_empty() {
            self.current = Some(self.images.remove(0));
        }
        
        self.refill();
    }

    pub fn next(&mut self) -> Option<&PathBuf> {
        if let Some(current) = &self.current {
            self.tail.push_back(current.clone());
        }

        self.current = self.buffer.pop_front();
        self.refill();
        
        // If current is still None but buffer has items (from cycling), get one
        if self.current.is_none() && !self.buffer.is_empty() {
            self.current = self.buffer.pop_front();
        }
        
        self.current.as_ref()
    }

    pub fn previous(&mut self) -> Option<&PathBuf> {
        if let Some(current) = &self.current {
            self.buffer.push_front(current.clone());
        }

        self.current = self.tail.pop_back();
        self.current.as_ref()
    }

    fn refill(&mut self) {
        // Use remove(0) to maintain order for ascending/descending sorts
        while self.buffer.len() < self.size && !self.images.is_empty() {
            let image = self.images.remove(0);
            self.buffer.push_back(image);
        }
        
        // If buffer is still empty and we have no more images, but we have a tail (history),
        // restart the queue by moving all images from tail back to the pool
        if self.buffer.is_empty() && self.images.is_empty() && !self.tail.is_empty() {
            log::debug!("Queue exhausted, restarting cycle with {} images", self.tail.len());
            
            // Move all tail images back to the main pool for reprocessing
            let mut restart_images: Vec<PathBuf> = self.tail.drain(..).collect();
            
            // Re-sort according to our sorting mode
            match self.sorting {
                Sorting::Random => {
                    let mut rng = rand::thread_rng();
                    restart_images.shuffle(&mut rng);
                }
                Sorting::Ascending => {
                    restart_images.sort();
                }
                Sorting::Descending => {
                    restart_images.sort_by(|a, b| b.cmp(a));
                }
            }
            
            // Put them back in images pool and refill buffer
            self.images = restart_images;
            // Use remove(0) to maintain order
            while self.buffer.len() < self.size && !self.images.is_empty() {
                let image = self.images.remove(0);
                self.buffer.push_back(image);
            }
        }
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.size
    }

    pub fn current_image(&self) -> Option<&PathBuf> {
        self.current.as_ref()
    }

    pub fn current_position(&self) -> usize {
        self.tail.len()
    }

    pub fn size(&self) -> usize {
        self.tail.len() + self.buffer.len() + if self.current.is_some() { 1 } else { 0 }
    }

    pub fn get_sorting(&self) -> Sorting {
        self.sorting.clone()
    }

    pub fn get_all_images(&self) -> Vec<PathBuf> {
        let mut all_images = Vec::new();
        
        // Add images in the order they would be processed
        // This includes tail (history), current, and buffer (future)
        all_images.extend(self.tail.iter().cloned());
        if let Some(current) = &self.current {
            all_images.push(current.clone());
        }
        all_images.extend(self.buffer.iter().cloned());
        all_images.extend(self.images.iter().cloned());
        
        all_images
    }

    pub fn set_position(&mut self, position: usize) -> bool {
        // Clear current state
        self.current = None;
        self.buffer.clear();
        self.tail.clear();
        
        // Rebuild the queue to the specified position
        let total_images = self.images.len();
        if position >= total_images {
            return false;
        }
        
        // Move images to tail up to the position
        for _ in 0..position {
            if !self.images.is_empty() {
                let image = self.images.remove(0);
                self.tail.push_back(image);
            }
        }
        
        // Set current image
        if !self.images.is_empty() {
            let image = self.images.remove(0);
            self.current = Some(image);
        }
        
        // Refill buffer
        self.refill();
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_queue_cycling_ascending() {
        let images = vec![
            PathBuf::from("/test/1.jpg"),
            PathBuf::from("/test/2.jpg"),
            PathBuf::from("/test/3.jpg"),
        ];
        
        let mut queue = Queue::new(2, Sorting::Ascending, images).unwrap();
        
        // First image should be 1.jpg (current)
        assert_eq!(queue.current_image(), Some(&PathBuf::from("/test/1.jpg")));
        
        // Next should be 2.jpg  
        assert_eq!(queue.next(), Some(&PathBuf::from("/test/2.jpg")));
        
        // Next should be 3.jpg
        assert_eq!(queue.next(), Some(&PathBuf::from("/test/3.jpg")));
        
        // Next should cycle back to 1.jpg
        assert_eq!(queue.next(), Some(&PathBuf::from("/test/1.jpg")));
        
        // Continue cycling
        assert_eq!(queue.next(), Some(&PathBuf::from("/test/2.jpg")));
    }
    
    #[test]
    fn test_queue_cycling_random() {
        let images = vec![
            PathBuf::from("/test/a.jpg"),
            PathBuf::from("/test/b.jpg"),
            PathBuf::from("/test/c.jpg"),
        ];
        
        let mut queue = Queue::new(2, Sorting::Random, images.clone()).unwrap();
        
        let mut seen_images = std::collections::HashSet::new();
        
        // Go through more than the original number of images to verify cycling
        for _ in 0..10 {
            if let Some(image) = queue.next() {
                seen_images.insert(image.clone());
                // Every image should be one of the original ones
                assert!(images.contains(image), "Image {:?} was not in original list", image);
            }
        }
        
        // We should have seen all original images at least once due to cycling
        assert_eq!(seen_images.len(), 3);
    }
    
    #[test] 
    fn test_queue_never_exhausted() {
        let images = vec![
            PathBuf::from("/test/single.jpg"),
        ];
        
        let mut queue = Queue::new(1, Sorting::Ascending, images).unwrap();
        
        // First image
        assert_eq!(queue.current_image(), Some(&PathBuf::from("/test/single.jpg")));
        
        // Should cycle indefinitely on the same image
        for _ in 0..5 {
            assert_eq!(queue.next(), Some(&PathBuf::from("/test/single.jpg")));
        }
    }
}
